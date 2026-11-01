//! Pluggable storage for PBF files.
//!
//! Production uses [`LogosctlStorage`] to call a persistent Logos Storage
//! module daemon. Tests use [`InMemoryStorage`].

use crate::error::SdkError;
use crate::geofabrik::backoff_delays;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt};
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

// ── logosctl backend ──────────────────────────────────────────────────────

/// Drives the current Logos Storage API through a running `logosctl` daemon.
pub struct LogosctlStorage {
    pub bin: PathBuf,
    pub config_dir: PathBuf,
    pub max_retries: u32,
    pub timeout: std::time::Duration,
}

async fn with_timeout<T, F>(
    duration: std::time::Duration,
    label: &str,
    future: F,
) -> Result<T, SdkError>
where
    F: std::future::Future<Output = Result<T, anyhow::Error>>,
{
    tokio::time::timeout(duration, future)
        .await
        .map_err(|_| SdkError::Other(anyhow::anyhow!("timed out waiting for {label}")))?
        .map_err(|e| SdkError::Other(anyhow::anyhow!("{label}: {e}")))
}

fn require_success_event(name: &str, payload: &serde_json::Value) -> Result<(), SdkError> {
    if payload.get("success").and_then(|v| v.as_bool()) == Some(true) {
        Ok(())
    } else {
        Err(SdkError::Other(anyhow::anyhow!("{name} failed: {payload}")))
    }
}

impl LogosctlStorage {
    pub fn new(bin: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            bin,
            config_dir,
            max_retries: 5,
            timeout: std::time::Duration::from_secs(300),
        }
    }

    fn base_args(&self) -> Vec<String> {
        vec![
            "--config-dir".into(),
            self.config_dir.to_string_lossy().into(),
        ]
    }

    pub(crate) fn command_for_call(&self, method: &str, values: &[String]) -> Vec<String> {
        let mut args = self.base_args();
        args.push("call".into());
        args.push("storage_module".into());
        args.push(method.into());
        args.extend(values.iter().cloned());
        args
    }

    pub(crate) fn parse_call_result(stdout: &str) -> Result<serde_json::Value, SdkError> {
        let value: serde_json::Value = serde_json::from_str(stdout.trim())?;
        if value.get("status").and_then(|v| v.as_str()) == Some("ok") {
            return Ok(value
                .get("result")
                .cloned()
                .unwrap_or(serde_json::Value::Null));
        }
        Err(SdkError::Other(anyhow::anyhow!(
            "logosctl call failed: {stdout}"
        )))
    }

    pub(crate) fn parse_upload_result(stdout: &str) -> Result<String, SdkError> {
        let result = Self::parse_call_result(stdout)?;
        if result.get("success").and_then(|v| v.as_bool()) != Some(true) {
            return Err(SdkError::Other(anyhow::anyhow!(
                "storage upload rejected: {result}"
            )));
        }
        result
            .get("value")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| {
                SdkError::Other(anyhow::anyhow!("upload result has no session ID: {result}"))
            })
    }

    pub(crate) fn parse_event(line: &str) -> Result<Option<(String, serde_json::Value)>, SdkError> {
        let event: serde_json::Value = serde_json::from_str(line.trim())?;
        let Some(name) = event.get("event").and_then(|v| v.as_str()) else {
            return Ok(None);
        };
        let arg0 = event
            .get("data")
            .and_then(|v| v.get("arg0"))
            .and_then(|v| v.as_str());
        let payload = match arg0 {
            Some(s) => serde_json::from_str(s)?,
            None => event
                .get("data")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
        };
        Ok(Some((name.to_owned(), payload)))
    }

    pub(crate) fn matching_event(
        stdout: &str,
        event_name: &str,
        session_id: &str,
    ) -> Result<Option<serde_json::Value>, SdkError> {
        for line in stdout.lines() {
            if let Some((name, payload)) = Self::parse_event(line)? {
                if name == event_name
                    && payload.get("sessionId").and_then(|v| v.as_str()) == Some(session_id)
                {
                    return Ok(Some(payload));
                }
            }
        }
        Ok(None)
    }

    async fn run(&self, args: &[String]) -> Result<String, SdkError> {
        let out = tokio::process::Command::new(&self.bin)
            .args(args)
            .output()
            .await
            .map_err(|e| SdkError::Other(anyhow::anyhow!("spawn logosctl: {e}")))?;
        let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        if !out.status.success() {
            return Err(SdkError::Other(anyhow::anyhow!(
                "logosctl failed ({}): {}{}",
                out.status,
                stdout,
                String::from_utf8_lossy(&out.stderr)
            )));
        }
        Ok(stdout)
    }

    async fn call(&self, method: &str, values: &[String]) -> Result<serde_json::Value, SdkError> {
        let args = self.command_for_call(method, values);
        Self::parse_call_result(&self.run(&args).await?)
    }

    async fn call_output(&self, method: &str, values: &[String]) -> Result<String, SdkError> {
        let args = self.command_for_call(method, values);
        self.run(&args).await
    }

    async fn ensure_running(&self) -> Result<(), SdkError> {
        let running = self.call("isRunning", &[]).await?;
        if running.as_bool() == Some(true) {
            return Ok(());
        }
        let _ = self.call("start", &[]).await?;
        let until = tokio::time::Instant::now() + self.timeout;
        loop {
            if self.call("isRunning", &[]).await?.as_bool() == Some(true) {
                return Ok(());
            }
            if tokio::time::Instant::now() >= until {
                return Err(SdkError::Other(anyhow::anyhow!(
                    "storage module start timed out"
                )));
            }
            tokio::time::sleep(std::time::Duration::from_millis(250)).await;
        }
    }
}

#[async_trait]
impl StorageBackend for LogosctlStorage {
    fn name(&self) -> &'static str {
        "logosctl"
    }

    async fn put(&self, file: &Path, name: &str) -> Result<PutReceipt, SdkError> {
        let _ = name;
        let delays = backoff_delays(self.max_retries);
        let mut last: Option<SdkError> = None;
        for (attempt, delay) in delays.into_iter().enumerate() {
            if attempt > 0 {
                tokio::time::sleep(delay).await;
            }
            match async {
                self.ensure_running().await?;
                let mut watch_args = self.base_args();
                watch_args.extend(["watch".into(), "storage_module".into()]);
                let mut watcher = tokio::process::Command::new(&self.bin)
                    .args(&watch_args)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|e| SdkError::Other(anyhow::anyhow!("spawn logosctl watch: {e}")))?;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let started = tokio::time::Instant::now();
                let result = self
                    .call_output(
                        "uploadUrl",
                        &[
                            file.to_string_lossy().into_owned(),
                            "65536".into(),
                            "true".into(),
                        ],
                    )
                    .await;
                let session = match result {
                    Ok(output) => match Self::parse_upload_result(&output) {
                        Ok(session) => session,
                        Err(e) => {
                            let _ = watcher.kill().await;
                            return Err(e);
                        }
                    },
                    Err(e) => {
                        let _ = watcher.kill().await;
                        return Err(e);
                    }
                };
                let stdout = watcher.stdout.take().expect("piped stdout");
                let mut lines = tokio::io::BufReader::new(stdout).lines();
                let wait = async {
                    while let Some(line) = lines.next_line().await? {
                        if let Some(payload) =
                            Self::matching_event(&line, "storageUploadDone", &session)?
                        {
                            require_success_event("storageUploadDone", &payload)
                                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                            let cid = payload
                                .get("cid")
                                .and_then(|v| v.as_str())
                                .filter(|s| !s.is_empty())
                                .ok_or_else(|| anyhow::anyhow!("upload event has no CID"))?;
                            return Ok::<_, anyhow::Error>(cid.to_owned());
                        }
                    }
                    anyhow::bail!("storage watch ended before upload completion")
                };
                let got = with_timeout(
                    self.timeout.saturating_sub(started.elapsed()),
                    &format!("storage upload event for session {session}"),
                    wait,
                )
                .await;
                let _ = watcher.kill().await;
                got
            }
            .await
            {
                Ok(cid) => {
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
        let delays = backoff_delays(self.max_retries);
        let mut last: Option<SdkError> = None;
        for (attempt, delay) in delays.into_iter().enumerate() {
            if attempt > 0 {
                tokio::time::sleep(delay).await;
            }
            match async {
                self.ensure_running().await?;
                let mut watch_args = self.base_args();
                watch_args.extend(["watch".into(), "storage_module".into()]);
                let mut watcher = tokio::process::Command::new(&self.bin)
                    .args(&watch_args)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|e| SdkError::Other(anyhow::anyhow!("spawn logosctl watch: {e}")))?;
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                let started = tokio::time::Instant::now();
                let result = self
                    .call(
                        "downloadToUrl",
                        &[
                            cid.to_owned(),
                            dest.to_string_lossy().into_owned(),
                            "false".into(),
                            "65536".into(),
                            "false".into(),
                            "true".into(),
                        ],
                    )
                    .await;
                let session = match result {
                    Ok(v) => {
                        if v.get("success").and_then(|v| v.as_bool()) == Some(false) {
                            let _ = watcher.kill().await;
                            return Err(SdkError::Other(anyhow::anyhow!(
                                "downloadToUrl rejected: {v}"
                            )));
                        }
                        v.get("value")
                            .and_then(|v| v.as_str())
                            .map(str::to_owned)
                            .ok_or_else(|| {
                                SdkError::Other(anyhow::anyhow!(
                                    "downloadToUrl response has no session ID"
                                ))
                            })?
                    }
                    Err(e) => {
                        let _ = watcher.kill().await;
                        return Err(e);
                    }
                };
                let stdout = watcher.stdout.take().expect("piped stdout");
                let mut lines = tokio::io::BufReader::new(stdout).lines();
                let wait = async {
                    while let Some(line) = lines.next_line().await? {
                        if let Some(payload) =
                            Self::matching_event(&line, "storageDownloadDone", &session)?
                        {
                            require_success_event("storageDownloadDone", &payload)
                                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                            return Ok::<_, anyhow::Error>(());
                        }
                    }
                    anyhow::bail!("storage watch ended before download completion")
                };
                let result = with_timeout(
                    self.timeout.saturating_sub(started.elapsed()),
                    &format!("storage download event for session {session}"),
                    wait,
                )
                .await;
                let _ = watcher.kill().await;
                result?;
                if tokio::fs::metadata(dest).await.is_err() {
                    return Err(SdkError::Other(anyhow::anyhow!(
                        "storage reported download success but {} is missing",
                        dest.display()
                    )));
                }
                Ok(())
            }
            .await
            {
                Ok(()) => return Ok(()),
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
    fn logosctl_builds_expected_argv() {
        let st = LogosctlStorage::new(PathBuf::from("logosctl"), PathBuf::from("/tmp/session"));
        let args = st.command_for_call(
            "uploadUrl",
            &["/tmp/x.pbf".into(), "65536".into(), "true".into()],
        );
        assert_eq!(
            args,
            [
                "--config-dir",
                "/tmp/session",
                "call",
                "storage_module",
                "uploadUrl",
                "/tmp/x.pbf",
                "65536",
                "true"
            ]
        );
    }

    #[test]
    fn logosctl_parses_nested_results_and_upload_session() {
        let response = r#"{"method":"uploadUrl","module":"storage_module","result":{"error":null,"success":true,"value":"session-9"},"status":"ok"}"#;
        assert_eq!(
            LogosctlStorage::parse_upload_result(response).unwrap(),
            "session-9"
        );
        let fail = r#"{"method":"uploadUrl","module":"storage_module","result":{"error":"disk full","success":false,"value":null},"status":"ok"}"#;
        assert!(LogosctlStorage::parse_upload_result(fail).is_err());
        assert!(
            LogosctlStorage::parse_call_result(r#"{"status":"error","message":"no"}"#).is_err()
        );
    }

    #[test]
    fn event_matching_correlates_session_and_failure() {
        let events = concat!(
            r#"{"data":{"arg0":"{\"cid\":\"wrong\",\"sessionId\":\"1\",\"success\":true}"},"event":"storageUploadDone"}"#,
            "\n",
            r#"{"data":{"arg0":"{\"cid\":\"right\",\"sessionId\":\"2\",\"success\":true}"},"event":"storageUploadDone"}"#
        );
        let got = LogosctlStorage::matching_event(events, "storageUploadDone", "2")
            .unwrap()
            .unwrap();
        assert_eq!(got["cid"], "right");
        let failed = r#"{"data":{"arg0":"{\"error\":\"offline\",\"sessionId\":\"2\",\"success\":false}"},"event":"storageDownloadDone"}"#;
        let event = LogosctlStorage::matching_event(failed, "storageDownloadDone", "2")
            .unwrap()
            .unwrap();
        assert_eq!(event["success"], false);
        assert!(require_success_event("storageDownloadDone", &event).is_err());
    }

    #[tokio::test]
    async fn event_wait_timeout_is_bounded() {
        let result = with_timeout(
            std::time::Duration::from_millis(1),
            "storage event",
            std::future::pending::<Result<(), anyhow::Error>>(),
        )
        .await;
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("timed out waiting"));
    }

    #[tokio::test]
    #[ignore = "requires a running Logos Storage node; set LOGOSCTL_BIN, LOGOSCTL_CONFIG_DIR, and STORAGE_ROUNDTRIP_FILE"]
    async fn live_logosctl_roundtrip() {
        let bin = PathBuf::from(std::env::var_os("LOGOSCTL_BIN").unwrap());
        let config = PathBuf::from(std::env::var_os("LOGOSCTL_CONFIG_DIR").unwrap());
        let src = PathBuf::from(std::env::var_os("STORAGE_ROUNDTRIP_FILE").unwrap());
        let dest = PathBuf::from(std::env::var_os("STORAGE_ROUNDTRIP_DEST").unwrap());
        let storage = LogosctlStorage::new(bin, config);
        let receipt = storage.put(&src, "roundtrip").await.unwrap();
        assert_eq!(receipt.size, tokio::fs::metadata(&src).await.unwrap().len());
        storage.get(&receipt.cid, &dest).await.unwrap();
        assert_eq!(
            tokio::fs::read(src).await.unwrap(),
            tokio::fs::read(dest).await.unwrap()
        );
        println!("roundtrip cid={} size={}", receipt.cid, receipt.size);
    }
}
