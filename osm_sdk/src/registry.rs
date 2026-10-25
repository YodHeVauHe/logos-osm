//! On-chain OSM registry client.
//!
//! Reads are off-chain: the registry state lives in a single PDA account, so
//! queries fetch the account bytes via `lgs wallet account get` and borsh-
//! decode them locally, then filter by region / parent / CID. Writes go
//! through the SPEL-generated instruction CLI (`osm_registry_cli`), which
//! builds and submits transactions from the committed IDL.

use crate::error::SdkError;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use osm_registry_core::{Registry, RegionEntry, MAX_BATCH};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Stdio;

#[async_trait]
pub trait RegistryClient: Send + Sync {
    /// Derive the registry PDA address.
    async fn pda(&self) -> Result<String, SdkError>;
    /// Create the registry PDA if absent (permissionless, idempotent).
    async fn init_registry(&self) -> Result<(), SdkError>;
    /// Upsert entries. One entry → `register_region`; more → `register_batch`.
    /// Entries must be pre-validated (`osm_registry_core::validate_entry`).
    async fn register(&self, entries: &[RegionEntry], registered_by: [u8; 32])
        -> Result<(), SdkError>;
    /// Full registry state; `None` when the PDA is not initialized yet.
    async fn fetch_all(&self) -> Result<Option<BTreeMap<String, RegionEntry>>, SdkError>;
}

#[async_trait]
impl<T: RegistryClient + ?Sized> RegistryClient for std::sync::Arc<T> {
    async fn pda(&self) -> Result<String, SdkError> {
        (**self).pda().await
    }
    async fn init_registry(&self) -> Result<(), SdkError> {
        (**self).init_registry().await
    }
    async fn register(
        &self,
        entries: &[RegionEntry],
        registered_by: [u8; 32],
    ) -> Result<(), SdkError> {
        (**self).register(entries, registered_by).await
    }
    async fn fetch_all(&self) -> Result<Option<BTreeMap<String, RegionEntry>>, SdkError> {
        (**self).fetch_all().await
    }
}

// ── Batch payload (hex-encoded borsh Vec<RegionEntry>) ────────────────────

/// Encode entries for the `register_batch` instruction. Region paths contain
/// `/`, URLs contain `:` — text packing is unsafe, so the batch rides as a
/// single hex string of the borsh-encoded vec.
pub fn batch_payload(entries: &[RegionEntry]) -> Result<String, SdkError> {
    if entries.is_empty() {
        return Err(SdkError::Other(anyhow::anyhow!("batch payload is empty")));
    }
    if entries.len() > MAX_BATCH {
        return Err(SdkError::BatchTooBig(entries.len()));
    }
    let bytes = borsh::to_vec(entries)
        .map_err(|e| SdkError::Other(anyhow::anyhow!("borsh encode batch: {e}")))?;
    Ok(hex::encode(bytes))
}

pub fn parse_batch_payload(hex_str: &str) -> Result<Vec<RegionEntry>, SdkError> {
    let bytes = hex::decode(hex_str)
        .map_err(|e| SdkError::Other(anyhow::anyhow!("batch payload is not hex: {e}")))?;
    borsh::from_slice(&bytes)
        .map_err(|e| SdkError::Other(anyhow::anyhow!("batch payload borsh decode: {e}")))
}

// ── Off-chain query helpers (pure, over decoded state) ────────────────────

pub fn query_by_region<'a>(
    map: &'a BTreeMap<String, RegionEntry>,
    region: &str,
) -> Option<&'a RegionEntry> {
    map.get(region)
}

/// Entries under one decomposed country, ordered by timestamp ascending —
/// the spec's "entries are ordered by timestamp for version sorting".
pub fn query_by_parent<'a>(
    map: &'a BTreeMap<String, RegionEntry>,
    parent: &str,
) -> Vec<&'a RegionEntry> {
    let mut v: Vec<&RegionEntry> = map
        .values()
        .filter(|e| e.parent.as_deref() == Some(parent))
        .collect();
    v.sort_by_key(|e| e.timestamp);
    v
}

pub fn query_by_cid<'a>(
    map: &'a BTreeMap<String, RegionEntry>,
    cid: &str,
) -> Vec<&'a RegionEntry> {
    map.values().filter(|e| e.cid == cid).collect()
}

// ── SPEL CLI + lgs implementation ─────────────────────────────────────────

/// Talks to the chain by shelling out to the SPEL-generated
/// `osm_registry_cli` (transactions) and `lgs wallet` (account reads) —
/// the same boundaries the reference LP-0017 implementation uses.
pub struct SpelCliRegistryClient {
    pub cli_bin: PathBuf,
    pub idl_path: PathBuf,
    pub program_id: String,
    pub lgs_bin: PathBuf,
    pub wallet_home: PathBuf,
    pub signer_account_id: String,
}

impl SpelCliRegistryClient {
    fn cli_cmd(&self) -> tokio::process::Command {
        let mut c = tokio::process::Command::new(&self.cli_bin);
        c.arg("-i").arg(&self.idl_path).arg("-p").arg(&self.program_id);
        c.env("LEE_WALLET_HOME_DIR", &self.wallet_home);
        c
    }

    fn lgs_cmd(&self) -> tokio::process::Command {
        let mut c = tokio::process::Command::new(&self.lgs_bin);
        c.env("LEE_WALLET_HOME_DIR", &self.wallet_home);
        c
    }

    async fn run(cmd: &mut tokio::process::Command) -> Result<(String, String, bool), SdkError> {
        let out = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| SdkError::Other(anyhow::anyhow!("spawn: {e}")))?;
        Ok((
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
            out.status.success(),
        ))
    }

    /// Pull the `PDA registry -> <addr>` line out of `--dry-run=text` output.
    pub(crate) fn parse_pda_line(text: &str) -> Option<String> {
        for line in text.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("PDA registry") {
                let rest = rest.trim().trim_start_matches('\u{2192}').trim_start_matches("->").trim_start();
                if let Some(addr) = rest.split_whitespace().next() {
                    return Some(addr.to_string());
                }
            }
        }
        None
    }

    /// Parse `lgs wallet account get --raw` JSON: first `{` .. last `}`.
    /// An uninitialized account prints no JSON → `Ok(None)`.
    pub(crate) fn parse_account_get_json(stdout: &str) -> Result<Option<Vec<u8>>, SdkError> {
        let (start, end) = match (stdout.find('{'), stdout.rfind('}')) {
            (Some(s), Some(e)) if s <= e => (s, e),
            _ => return Ok(None),
        };
        let v: serde_json::Value = serde_json::from_str(&stdout[start..=end])
            .map_err(|e| SdkError::Other(anyhow::anyhow!("account get JSON: {e}")))?;
        match v.get("data").and_then(|d| d.as_str()) {
            Some(h) => {
                let bytes = hex::decode(h)
                    .map_err(|e| SdkError::Other(anyhow::anyhow!("account data hex: {e}")))?;
                Ok(Some(bytes))
            }
            None => Ok(None),
        }
    }
}

#[async_trait]
impl RegistryClient for SpelCliRegistryClient {
    async fn pda(&self) -> Result<String, SdkError> {
        let mut cmd = self.cli_cmd();
        cmd.args(["--dry-run=text", "init-registry", "--anchorer", &self.signer_account_id]);
        let (stdout, stderr, ok) = Self::run(&mut cmd).await?;
        if !ok {
            return Err(SdkError::Other(anyhow::anyhow!(
                "pda dry-run failed: {stdout}{stderr}"
            )));
        }
        Self::parse_pda_line(&format!("{stdout}{stderr}")).ok_or_else(|| {
            SdkError::Other(anyhow::anyhow!("no 'PDA registry' line in dry-run output"))
        })
    }

    async fn init_registry(&self) -> Result<(), SdkError> {
        let pda = self.pda().await?;
        if self.fetch_account_bytes(&pda).await?.is_some() {
            return Ok(());
        }
        let mut cmd = self.cli_cmd();
        cmd.args(["init-registry", "--anchorer", &self.signer_account_id]);
        let (stdout, stderr, ok) = Self::run(&mut cmd).await?;
        if !ok {
            return Err(SdkError::Other(anyhow::anyhow!(
                "init-registry failed: {stdout}{stderr}"
            )));
        }
        Ok(())
    }

    async fn register(
        &self,
        entries: &[RegionEntry],
        registered_by: [u8; 32],
    ) -> Result<(), SdkError> {
        if entries.is_empty() {
            return Ok(());
        }
        for e in entries {
            if let Err(code) = osm_registry_core::validate_entry(e) {
                return Err(SdkError::InvalidEntry { region: e.region.clone(), code });
            }
        }
        let by = hex::encode(registered_by);
        let mut cmd = self.cli_cmd();
        if entries.len() == 1 {
            let e = &entries[0];
            cmd.args([
                "register-region",
                "--region", &e.region,
                "--parent", e.parent.as_deref().unwrap_or(""),
                "--level", &e.level.to_string(),
                "--cid", &e.cid,
                "--source-url", &e.source_url,
                "--checksum", &e.checksum,
                "--version", &e.version,
                "--hosted", if e.hosted { "true" } else { "false" },
                "--timestamp", &e.timestamp.to_string(),
                "--registered-by", &by,
            ]);
        } else {
            if entries.len() > MAX_BATCH {
                return Err(SdkError::BatchTooBig(entries.len()));
            }
            let payload = batch_payload(entries)?;
            cmd.args([
                "register-batch",
                "--payload", &payload,
                "--registered-by", &by,
            ]);
        }
        let (stdout, stderr, ok) = Self::run(&mut cmd).await?;
        if !ok {
            return Err(SdkError::Other(anyhow::anyhow!(
                "register failed: {stdout}{stderr}"
            )));
        }
        Ok(())
    }

    async fn fetch_all(&self) -> Result<Option<BTreeMap<String, RegionEntry>>, SdkError> {
        let pda = self.pda().await?;
        match self.fetch_account_bytes(&pda).await? {
            None => Ok(None),
            Some(bytes) if bytes.is_empty() => Ok(None),
            Some(bytes) => {
                let reg = Registry::try_from_slice(&bytes).map_err(|e| {
                    SdkError::Other(anyhow::anyhow!("registry borsh decode: {e}"))
                })?;
                Ok(Some(reg.entries))
            }
        }
    }
}

impl SpelCliRegistryClient {
    async fn fetch_account_bytes(&self, pda: &str) -> Result<Option<Vec<u8>>, SdkError> {
        let mut cmd = self.lgs_cmd();
        cmd.args(["wallet", "--", "account", "get", "--account-id"])
            .arg(format!("Public/{pda}"))
            .arg("--raw");
        let (stdout, stderr, ok) = Self::run(&mut cmd).await?;
        if !ok && (stdout.contains("not found") || stderr.contains("not found")) {
            return Ok(None);
        }
        if !ok {
            return Err(SdkError::Other(anyhow::anyhow!(
                "lgs account get failed: {stdout}{stderr}"
            )));
        }
        Self::parse_account_get_json(&stdout)
    }
}

// ── Mock for tests and downstream integration tests ───────────────────────

#[cfg(any(test, feature = "test-util"))]
#[derive(Default)]
pub struct MockRegistryClient {
    pub state: std::sync::Mutex<Option<BTreeMap<String, RegionEntry>>>,
    pub register_calls: std::sync::Mutex<Vec<usize>>,
}

#[cfg(any(test, feature = "test-util"))]
impl MockRegistryClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_call_count(&self) -> usize {
        self.register_calls.lock().unwrap().len()
    }
}

#[cfg(any(test, feature = "test-util"))]
#[async_trait]
impl RegistryClient for MockRegistryClient {
    async fn pda(&self) -> Result<String, SdkError> {
        Ok("MockPdaAddress1111111111111111111111111111".into())
    }

    async fn init_registry(&self) -> Result<(), SdkError> {
        let mut st = self.state.lock().unwrap();
        if st.is_none() {
            *st = Some(BTreeMap::new());
        }
        Ok(())
    }

    async fn register(
        &self,
        entries: &[RegionEntry],
        _registered_by: [u8; 32],
    ) -> Result<(), SdkError> {
        if entries.len() > MAX_BATCH {
            return Err(SdkError::BatchTooBig(entries.len()));
        }
        self.register_calls.lock().unwrap().push(entries.len());
        let mut st = self.state.lock().unwrap();
        let map = st.get_or_insert_with(BTreeMap::new);
        for e in entries {
            if let Err(code) = osm_registry_core::validate_entry(e) {
                return Err(SdkError::InvalidEntry { region: e.region.clone(), code });
            }
            map.insert(e.region.clone(), e.clone());
        }
        Ok(())
    }

    async fn fetch_all(&self) -> Result<Option<BTreeMap<String, RegionEntry>>, SdkError> {
        Ok(self.state.lock().unwrap().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(region: &str, parent: Option<&str>, ts: i64) -> RegionEntry {
        RegionEntry {
            region: region.into(),
            parent: parent.map(|p| p.into()),
            level: if parent.is_some() { 1 } else { 0 },
            cid: format!("bafk-{region}").chars().take(128).collect(),
            source_url: format!("https://download.geofabrik.de/{region}-latest.osm.pbf"),
            checksum: "35cff894271b8f56ddb81911dcffe089".into(),
            version: "261024".into(),
            hosted: true,
            timestamp: ts,
            registered_by: [1u8; 32],
            record_version: 1,
        }
    }

    #[test]
    fn batch_payload_roundtrip() {
        let entries = vec![entry("germany", None, 1), entry("us/california", Some("us"), 2)];
        let hex_str = batch_payload(&entries).unwrap();
        let back = parse_batch_payload(&hex_str).unwrap();
        assert_eq!(back, entries);
    }

    #[test]
    fn batch_payload_rejects_empty() {
        assert!(batch_payload(&[]).is_err());
    }

    #[test]
    fn query_by_parent_filters_and_sorts_by_timestamp() {
        let mut map = BTreeMap::new();
        map.insert("india/southern-zone".into(), entry("india/southern-zone", Some("india"), 30));
        map.insert("india/central-zone".into(), entry("india/central-zone", Some("india"), 10));
        map.insert("india/western-zone".into(), entry("india/western-zone", Some("india"), 20));
        map.insert("germany".into(), entry("germany", None, 5));
        let v = query_by_parent(&map, "india");
        assert_eq!(
            v.iter().map(|e| e.timestamp).collect::<Vec<_>>(),
            vec![10, 20, 30]
        );
    }

    #[test]
    fn query_by_cid_matches_multiple() {
        let mut map = BTreeMap::new();
        map.insert("a".into(), entry("a", None, 1));
        let mut b = entry("b", None, 2);
        b.cid = map["a"].cid.clone();
        map.insert("b".into(), b);
        assert_eq!(query_by_cid(&map, &map["a"].cid).len(), 2);
    }

    #[tokio::test]
    async fn mock_client_registers_and_fetches() {
        let mock = MockRegistryClient::new();
        mock.init_registry().await.unwrap();
        mock.register(&[entry("germany", None, 1)], [0u8; 32]).await.unwrap();
        mock.register(
            &[entry("a", None, 2), entry("b", None, 3), entry("c", None, 4)],
            [0u8; 32],
        )
        .await
        .unwrap();
        let all = mock.fetch_all().await.unwrap().unwrap();
        assert_eq!(all.len(), 4);
        assert_eq!(mock.register_call_count(), 2);
    }

    #[test]
    fn parse_pda_line_variants() {
        assert_eq!(
            SpelCliRegistryClient::parse_pda_line("PDA registry \u{2192} HvCtoPL6RvYmqg8m7qkRqu4e2b1dHt4XsU2dz8eixdwe"),
            Some("HvCtoPL6RvYmqg8m7qkRqu4e2b1dHt4XsU2dz8eixdwe".into())
        );
        assert_eq!(
            SpelCliRegistryClient::parse_pda_line("PDA registry -> SomeAddr22222222222222222222222222222"),
            Some("SomeAddr22222222222222222222222222222".into())
        );
        assert_eq!(SpelCliRegistryClient::parse_pda_line("nothing here"), None);
    }

    #[test]
    fn parse_account_get_json_variants() {
        assert!(SpelCliRegistryClient::parse_account_get_json("Account is Uninitialized").unwrap().is_none());
        let j = r#"$ lgs wallet account get
{
  "data": "010203",
  "owner": "abc"
}"#;
        assert_eq!(
            SpelCliRegistryClient::parse_account_get_json(j).unwrap(),
            Some(vec![1, 2, 3])
        );
    }
}
