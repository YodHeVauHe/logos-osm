//! On-chain OSM registry client.
//!
//! Registry transactions use the LEZ v0.3.1 wallet client. Reads fetch the
//! public registry shard and decode its Borsh `Registry` bytes locally.

use crate::error::SdkError;
use async_trait::async_trait;
use borsh::BorshDeserialize;
use osm_registry_core::{RegionEntry, Registry, MAX_BATCH};
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
    async fn register(
        &self,
        entries: &[RegionEntry],
        registered_by: [u8; 32],
    ) -> Result<(), SdkError>;
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
    sort_entries(&mut v);
    v
}

pub fn query_by_cid<'a>(map: &'a BTreeMap<String, RegionEntry>, cid: &str) -> Vec<&'a RegionEntry> {
    let mut v: Vec<&RegionEntry> = map.values().filter(|e| e.cid == cid).collect();
    sort_entries(&mut v);
    v
}

fn sort_entries(entries: &mut [&RegionEntry]) {
    entries.sort_by(|a, b| (a.timestamp, a.region.as_str()).cmp(&(b.timestamp, b.region.as_str())));
}

// ── LEZ v0.3.1 wallet CLI implementation ──────────────────────────────────

/// Talks to the chain through `osm_registry_cli`, which uses LEZ v0.3.1's
/// WalletCore, RPC client, and public transaction builder.
pub struct LezWalletRegistryClient {
    pub cli_bin: PathBuf,
    pub program_id: String,
    pub rpc_url: String,
    pub wallet_home: PathBuf,
    pub signer_account_id: String,
}

/// Kept as a source compatibility name for clients that used the old type name.
pub type SpelCliRegistryClient = LezWalletRegistryClient;

impl LezWalletRegistryClient {
    fn cli_cmd(&self) -> tokio::process::Command {
        let mut c = tokio::process::Command::new(&self.cli_bin);
        c.arg("--rpc-url")
            .arg(&self.rpc_url)
            .arg("--program-id")
            .arg(&self.program_id);
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

    pub(crate) fn parse_raw_shard(text: &str) -> Result<Option<Vec<u8>>, SdkError> {
        let text = text.trim();
        if text.is_empty() || text == "null" {
            return Ok(None);
        }
        hex::decode(text)
            .map(Some)
            .map_err(|e| SdkError::Other(anyhow::anyhow!("registry shard is not hex: {e}")))
    }
}

#[async_trait]
impl RegistryClient for LezWalletRegistryClient {
    async fn pda(&self) -> Result<String, SdkError> {
        let mut cmd = self.cli_cmd();
        cmd.arg("pda");
        let (stdout, stderr, ok) = Self::run(&mut cmd).await?;
        if !ok {
            return Err(SdkError::Other(anyhow::anyhow!(
                "registry PDA query failed: {stdout}{stderr}"
            )));
        }
        Ok(stdout.trim().to_owned())
    }

    async fn init_registry(&self) -> Result<(), SdkError> {
        if self.fetch_all().await?.is_some() {
            return Ok(());
        }
        let mut cmd = self.cli_cmd();
        cmd.args(["init", "--payer", &self.signer_account_id]);
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
                return Err(SdkError::InvalidEntry {
                    region: e.region.clone(),
                    code,
                });
            }
        }
        if entries.len() > MAX_BATCH {
            return Err(SdkError::BatchTooBig(entries.len()));
        }
        let mut entries = entries.to_vec();
        for entry in &mut entries {
            entry.registered_by = registered_by;
        }
        let mut cmd = self.cli_cmd();
        if entries.len() == 1 {
            let encoded = serde_json::to_string(&entries[0])
                .map_err(|e| SdkError::Other(anyhow::anyhow!("entry JSON encode: {e}")))?;
            cmd.args([
                "register",
                "--payer",
                &self.signer_account_id,
                "--entry-json",
                &encoded,
            ]);
        } else {
            let encoded = serde_json::to_string(&entries)
                .map_err(|e| SdkError::Other(anyhow::anyhow!("batch JSON encode: {e}")))?;
            cmd.args([
                "batch",
                "--payer",
                &self.signer_account_id,
                "--entries-json",
                &encoded,
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
        match self.fetch_account_bytes().await? {
            None => Ok(None),
            Some(bytes) if bytes.is_empty() => Ok(None),
            Some(bytes) => {
                let reg = Registry::try_from_slice(&bytes)
                    .map_err(|e| SdkError::Other(anyhow::anyhow!("registry borsh decode: {e}")))?;
                Ok(Some(reg.entries))
            }
        }
    }
}

impl LezWalletRegistryClient {
    async fn fetch_account_bytes(&self) -> Result<Option<Vec<u8>>, SdkError> {
        let mut cmd = self.cli_cmd();
        cmd.args(["get", "--raw"]);
        let (stdout, stderr, ok) = Self::run(&mut cmd).await?;
        if !ok {
            return Err(SdkError::Other(anyhow::anyhow!(
                "registry shard query failed: {stdout}{stderr}"
            )));
        }
        Self::parse_raw_shard(&stdout)
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
                return Err(SdkError::InvalidEntry {
                    region: e.region.clone(),
                    code,
                });
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
        let entries = vec![
            entry("germany", None, 1),
            entry("us/california", Some("us"), 2),
        ];
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
        map.insert(
            "india/southern-zone".into(),
            entry("india/southern-zone", Some("india"), 30),
        );
        map.insert(
            "india/central-zone".into(),
            entry("india/central-zone", Some("india"), 10),
        );
        map.insert(
            "india/western-zone".into(),
            entry("india/western-zone", Some("india"), 20),
        );
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
        map.insert("a".into(), entry("a", None, 3));
        let mut b = entry("b", None, 1);
        b.cid = map["a"].cid.clone();
        map.insert("b".into(), b);
        let matches = query_by_cid(&map, &map["a"].cid);
        assert_eq!(matches.len(), 2);
        assert_eq!(
            matches.iter().map(|e| e.timestamp).collect::<Vec<_>>(),
            vec![1, 3]
        );
    }

    #[test]
    fn collection_queries_sort_by_timestamp_then_region_and_handle_empty() {
        let mut map = BTreeMap::new();
        map.insert("india/west".into(), entry("india/west", Some("india"), 2));
        map.insert("india/east".into(), entry("india/east", Some("india"), 1));
        map.insert("india/north".into(), entry("india/north", Some("india"), 1));

        let by_parent = query_by_parent(&map, "india");
        assert_eq!(
            by_parent
                .iter()
                .map(|e| e.region.as_str())
                .collect::<Vec<_>>(),
            vec!["india/east", "india/north", "india/west"]
        );
        let by_cid = query_by_cid(&map, "missing");
        assert!(by_cid.is_empty());

        let shared_cid = "bafy-shared";
        for entry in map.values_mut() {
            entry.cid = shared_cid.into();
        }
        assert_eq!(
            query_by_cid(&map, shared_cid)
                .iter()
                .map(|e| e.region.as_str())
                .collect::<Vec<_>>(),
            vec!["india/east", "india/north", "india/west"]
        );
    }

    #[tokio::test]
    async fn mock_client_registers_and_fetches() {
        let mock = MockRegistryClient::new();
        mock.init_registry().await.unwrap();
        mock.register(&[entry("germany", None, 1)], [0u8; 32])
            .await
            .unwrap();
        mock.register(
            &[
                entry("a", None, 2),
                entry("b", None, 3),
                entry("c", None, 4),
            ],
            [0u8; 32],
        )
        .await
        .unwrap();
        let all = mock.fetch_all().await.unwrap().unwrap();
        assert_eq!(all.len(), 4);
        assert_eq!(mock.register_call_count(), 2);
    }

    #[test]
    fn parse_v03_shard_response() {
        assert_eq!(
            LezWalletRegistryClient::parse_raw_shard("null\n").unwrap(),
            None
        );
        assert_eq!(
            LezWalletRegistryClient::parse_raw_shard("010203\n").unwrap(),
            Some(vec![1, 2, 3])
        );
        assert!(LezWalletRegistryClient::parse_raw_shard("not hex").is_err());
    }
}
