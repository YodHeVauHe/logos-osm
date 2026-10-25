//! Pluggable storage for PBF files.
//!
//! Production uses [`LogoscoreStorage`], which drives the headless
//! `logoscore` binary (`--load-modules storage_module`) — the same module the
//! Basecamp storage app hosts, used standalone so the CLI has no GUI
//! dependency. Tests use [`InMemoryStorage`].

use crate::error::SdkError;
use crate::geofabrik::backoff_delays;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PutReceipt {
    pub cid: String,
    pub size: u64,
}

#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store the bytes of `file` under `name`; returns the content ID.
    async fn put(&self, file: &Path, name: &str) -> Result<PutReceipt, SdkError>;
    /// Fetch `cid` to `dest`.
    async fn get(&self, cid: &str, dest: &Path) -> Result<(), SdkError>;
    fn name(&self) -> &'static str;
}

#[async_trait]
impl<T: StorageBackend + ?Sized> StorageBackend for std::sync::Arc<T> {
    async fn put(&self, file: &Path, name: &str) -> Result<PutReceipt, SdkError> {
        (**self).put(file, name).await
    }
    async fn get(&self, cid: &str, dest: &Path) -> Result<(), SdkError> {
        (**self).get(cid, dest).await
    }
    fn name(&self) -> &'static str {
        (**self).name()
    }
}

// ── In-memory test double ─────────────────────────────────────────────────

type ObjectMap = std::collections::HashMap<String, (Vec<u8>, String)>;

#[derive(Default, Clone)]
pub struct InMemoryStorage {
    objects: Arc<Mutex<ObjectMap>>,
    counter: Arc<Mutex<u64>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn object_count(&self) -> usize {
        self.objects.lock().await.len()
    }
}

#[async_trait]
impl StorageBackend for InMemoryStorage {
    fn name(&self) -> &'static str {
        "in-memory"
    }

    async fn put(&self, file: &Path, name: &str) -> Result<PutReceipt, SdkError> {
        let mut bytes = Vec::new();
        tokio::fs::File::open(file)
            .await?
            .read_to_end(&mut bytes)
            .await?;
        let size = bytes.len() as u64;
        let mut counter = self.counter.lock().await;
        *counter += 1;
        let cid = format!("bafk-inmem-{name}-{counter:06}");
        drop(counter);
        self.objects
            .lock()
            .await
            .insert(cid.clone(), (bytes, name.to_string()));
        Ok(PutReceipt { cid, size })
    }

    async fn get(&self, cid: &str, dest: &Path) -> Result<(), SdkError> {
        let objects = self.objects.lock().await;
        let (bytes, _) = objects
            .get(cid)
            .ok_or_else(|| SdkError::Other(anyhow::anyhow!("cid not found in storage: {cid}")))?;
        tokio::fs::write(dest, bytes).await?;
        Ok(())
    }
}

// ── logoscore headless backend ────────────────────────────────────────────

/// Drives the `logoscore` binary against the `storage_module`.
///
/// One invocation per operation keeps the process stateless; the exact
/// command strings live in one place so they can be aligned with the
/// installed logoscore build without touching control flow.
pub struct LogoscoreStorage {
    pub bin: PathBuf,
    pub modules_dir: PathBuf,
    pub storage_config: PathBuf,
    pub max_retries: u32,
}

impl LogoscoreStorage {
    pub fn new(bin: PathBuf, modules_dir: PathBuf, storage_config: PathBuf) -> Self {
        Self {
            bin,
            modules_dir,
            storage_config,
            max_retries: 5,
        }
    }

    /// Init + start + op + stop, as a single logoscore command sequence.
    fn base_args(&self) -> Vec<String> {
        vec![
            "-m".into(),
            self.modules_dir.to_string_lossy().into(),
            "--load-modules".into(),
            "storage_module".into(),
            "-c".into(),
            format!(
                "storage_module.init(@{})",
                self.storage_config.to_string_lossy()
            ),
            "-c".into(),
            "storage_module.start()".into(),
        ]
    }

    pub(crate) fn command_for_put(&self, file: &Path, name: &str) -> Vec<String> {
        let mut args = self.base_args();
        args.push("-c".into());
        args.push(format!(
            "storage_module.uploadFile({})",
            file.to_string_lossy()
        ));
        args.push("-c".into());
        args.push(format!("storage_module.nameObject({name:?})"));
        args.push("-c".into());
        args.push("storage_module.stop()".into());
        args
    }

    pub(crate) fn command_for_get(&self, cid: &str, dest: &Path) -> Vec<String> {
        let mut args = self.base_args();
        args.push("-c".into());
        args.push(format!(
            "storage_module.downloadFile({cid:?}, {})",
            dest.to_string_lossy()
        ));
        args.push("-c".into());
        args.push("storage_module.stop()".into());
        args
    }

    /// Parse the CID out of a logoscore JSON response line.
    pub(crate) fn parse_cid_output(stdout: &str) -> Result<String, SdkError> {
        // logoscore prints one JSON object per -c command; take the last
        // success carrying a non-empty value (the CID).
        for line in stdout.lines().rev() {
            let line = line.trim();
            if !line.starts_with('{') {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
                if v.get("success").and_then(|s| s.as_bool()).unwrap_or(false) {
                    if let Some(val) = v.get("value").and_then(|x| x.as_str()) {
                        if !val.is_empty() {
                            return Ok(val.to_string());
                        }
                    }
                }
            }
        }
        Err(SdkError::Other(anyhow::anyhow!(
            "no success value in logoscore output: {}",
            stdout.lines().last().unwrap_or("")
        )))
    }

    async fn run(&self, args: &[String]) -> Result<String, SdkError> {
        let out = tokio::process::Command::new(&self.bin)
            .args(args)
            .output()
            .await
            .map_err(|e| SdkError::Other(anyhow::anyhow!("spawn logoscore: {e}")))?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        if !out.status.success() {
            return Err(SdkError::Other(anyhow::anyhow!(
                "logoscore failed ({}): {}{}",
                out.status,
                stdout,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(stdout)
    }
}

#[async_trait]
impl StorageBackend for LogoscoreStorage {
    fn name(&self) -> &'static str {
        "logoscore"
    }

    async fn put(&self, file: &Path, name: &str) -> Result<PutReceipt, SdkError> {
        let args = self.command_for_put(file, name);
        let delays = backoff_delays(self.max_retries);
        let mut last: Option<SdkError> = None;
        for (attempt, delay) in delays.into_iter().enumerate() {
            if attempt > 0 {
                tokio::time::sleep(delay).await;
            }
            match self.run(&args).await {
                Ok(stdout) => {
                    let cid = Self::parse_cid_output(&stdout)?;
                    let size = tokio::fs::metadata(file)
                        .await
                        .map(|m| m.len())
                        .unwrap_or(0);
                    return Ok(PutReceipt { cid, size });
                }
                Err(e) => {
                    tracing::warn!(attempt, error = %e, "storage upload attempt failed");
                    last = Some(e);
                }
            }
        }
        Err(SdkError::RetriesExhausted {
            op: "storage upload",
            retries: self.max_retries,
            message: last.map(|e| e.to_string()).unwrap_or_default(),
        })
    }

    async fn get(&self, cid: &str, dest: &Path) -> Result<(), SdkError> {
        let args = self.command_for_get(cid, dest);
        let delays = backoff_delays(self.max_retries);
        let mut last: Option<SdkError> = None;
        for (attempt, delay) in delays.into_iter().enumerate() {
            if attempt > 0 {
                tokio::time::sleep(delay).await;
            }
            match self.run(&args).await {
                Ok(_) => {
                    if tokio::fs::metadata(dest).await.is_ok() {
                        return Ok(());
                    }
                    last = Some(SdkError::Other(anyhow::anyhow!(
                        "logoscore reported success but {} is missing",
                        dest.display()
                    )));
                }
                Err(e) => {
                    tracing::warn!(attempt, error = %e, "storage download attempt failed");
                    last = Some(e);
                }
            }
        }
        Err(SdkError::RetriesExhausted {
            op: "storage download",
            retries: self.max_retries,
            message: last.map(|e| e.to_string()).unwrap_or_default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn inmemory_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.bin");
        tokio::fs::write(&src, b"hello storage").await.unwrap();
        let st = InMemoryStorage::new();
        let r = st.put(&src, "test-region").await.unwrap();
        assert!(r.cid.starts_with("bafk-inmem-test-region-"));
        assert_eq!(r.size, 13);
        let dest = dir.path().join("out.bin");
        st.get(&r.cid, &dest).await.unwrap();
        assert_eq!(tokio::fs::read(&dest).await.unwrap(), b"hello storage");
    }

    #[test]
    fn backoff_delays_shape() {
        let d: Vec<u64> = backoff_delays(4).iter().map(|d| d.as_secs()).collect();
        assert_eq!(d, vec![1, 2, 4, 8]);
    }

    #[test]
    fn logoscore_builds_expected_command() {
        let st = LogoscoreStorage::new(
            PathBuf::from("/usr/bin/logoscore"),
            PathBuf::from("/opt/logos/modules"),
            PathBuf::from("/etc/storage.json"),
        );
        let args = st.command_for_put(Path::new("/tmp/x.pbf"), "germany");
        let joined = args.join(" ");
        assert!(joined.contains("--load-modules storage_module"));
        assert!(joined.contains("storage_module.init(@/etc/storage.json)"));
        assert!(joined.contains("storage_module.start()"));
        assert!(joined.contains("storage_module.uploadFile(/tmp/x.pbf)"));
        assert!(joined.contains("storage_module.stop()"));
        let args = st.command_for_get("bafk123", Path::new("/tmp/out.pbf"));
        assert!(args
            .join(" ")
            .contains("storage_module.downloadFile(\"bafk123\", /tmp/out.pbf)"));
    }

    #[test]
    fn logoscore_parses_cid_from_output() {
        let out = "{\"success\":true,\"value\":\"session-1\"}\n\
                   {\"success\":true,\"value\":\"bafkqacidonewcbyaaaa\"}\n\
                   {\"success\":true,\"value\":null}";
        assert_eq!(
            LogoscoreStorage::parse_cid_output(out).unwrap(),
            "bafkqacidonewcbyaaaa"
        );
        let bad = "{\"success\":false,\"error\":\"boom\"}";
        assert!(LogoscoreStorage::parse_cid_output(bad).is_err());
    }
}
