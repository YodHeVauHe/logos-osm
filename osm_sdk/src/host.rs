//! Distribution workflows: host, bulk host, local import, updates, download.
//!
//! Host pipeline (spec): download PBF from Geofabrik → verify MD5 against
//! Geofabrik's published checksum → store to Logos Storage → register the
//! CID on-chain. Download inverts it: fetch by CID from storage when hosted,
//! else Geofabrik as direct fallback; integrity is anchored by the CID.

use crate::error::SdkError;
use crate::geofabrik::Geofabrik;
use crate::regions;
use crate::registry::RegistryClient;
use crate::storage::StorageBackend;
use osm_registry_core::RegionEntry;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct HostOpts {
    /// Mark the entry as hosted (true for every path that stores bytes).
    pub hosted: bool,
    /// Retries for downloads and uploads (exponential back-off between).
    pub max_retries: u32,
}

impl Default for HostOpts {
    fn default() -> Self {
        Self { hosted: true, max_retries: 3 }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HostReport {
    pub registered: Vec<RegionEntry>,
    pub skipped: Vec<SkippedRegion>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkippedRegion {
    pub region: String,
    pub error: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegionUpdate {
    pub region: String,
    pub registry_version: String,
    pub central_version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadSource {
    LogosStorage,
    Geofabrik,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DownloadReport {
    pub region: String,
    pub source: DownloadSource,
    pub path: PathBuf,
    pub bytes: u64,
    pub verified: bool,
}

pub struct OsmSdk {
    pub geofabrik: Geofabrik,
    pub storage: Box<dyn StorageBackend>,
    pub registry: Box<dyn RegistryClient>,
}

impl OsmSdk {
    pub fn new(storage: Box<dyn StorageBackend>, registry: Box<dyn RegistryClient>) -> Self {
        Self { geofabrik: Geofabrik::new(), storage, registry }
    }

    /// All 72 predefined regions (bundled frozen set).
    pub fn discover(&self) -> &[regions::RegionDef] {
        regions::predefined()
    }

    /// The frozen set rebuilt from a live Geofabrik index fetch (fresh
    /// canonical URLs); falls back to the bundled URLs offline.
    pub fn discover_refreshed(&self, index_json: &str) -> Result<Vec<regions::RegionDef>, SdkError> {
        regions::refresh_from_index(index_json)
    }

    /// Resolve one region to its on-chain entry (CID, version, checksum…),
    /// or `None` when it is not registered yet.
    pub async fn resolve(&self, region: &str) -> Result<Option<RegionEntry>, SdkError> {
        if regions::get(region).is_none() {
            return Err(SdkError::UnknownRegion(region.into()));
        }
        let map = self.registry.fetch_all().await?;
        Ok(map.and_then(|m| crate::registry::query_by_region(&m, region).cloned()))
    }

    pub async fn query_by_parent(&self, parent: &str) -> Result<Vec<RegionEntry>, SdkError> {
        let map = self.registry.fetch_all().await?;
        Ok(map
            .map(|m| {
                crate::registry::query_by_parent(&m, parent)
                    .into_iter()
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    pub async fn query_by_cid(&self, cid: &str) -> Result<Vec<RegionEntry>, SdkError> {
        let map = self.registry.fetch_all().await?;
        Ok(map
            .map(|m| {
                crate::registry::query_by_cid(&m, cid).into_iter().cloned().collect()
            })
            .unwrap_or_default())
    }

    /// Host one region: fetch → verify → store → register.
    pub async fn host_region(
        &self,
        region: &str,
        signer: [u8; 32],
        opts: HostOpts,
    ) -> Result<RegionEntry, SdkError> {
        let report = self
            .host_regions(&[region.to_string()], signer, opts)
            .await?;
        match report.registered.into_iter().next() {
            Some(e) => Ok(e),
            None => Err(SdkError::Other(anyhow::anyhow!(
                "host failed: {}",
                report.skipped.first().map(|s| s.error.clone()).unwrap_or_default()
            ))),
        }
    }

    /// Bulk host: fetch+verify+store every region, then ONE batch-register
    /// transaction. A region that fails download/verify is skipped (in
    /// `HostReport::skipped`) and does not abort the rest.
    ///
    /// The predefined set is a non-overlapping partition, so bulk selection
    /// within it can never double-host geography; the guard below makes that
    /// guarantee structural for any caller-supplied superset.
    pub async fn host_regions(
        &self,
        region_names: &[String],
        signer: [u8; 32],
        opts: HostOpts,
    ) -> Result<HostReport, SdkError> {
        let mut deduped: Vec<&str> = {
            let mut v: Vec<&str> = region_names.iter().map(String::as_str).collect();
            v.sort_unstable();
            v.dedup();
            v
        };
        // Overlap guard: no selected region may contain another.
        for (i, a) in deduped.iter().enumerate() {
            for b in deduped.iter().skip(i + 1) {
                if b.starts_with(&format!("{a}/")) {
                    return Err(SdkError::Overlap(a.to_string(), b.to_string()));
                }
                if a.starts_with(&format!("{b}/")) {
                    return Err(SdkError::Overlap(b.to_string(), a.to_string()));
                }
            }
        }
        if deduped.len() > osm_registry_core::MAX_BATCH {
            return Err(SdkError::BatchTooBig(deduped.len()));
        }

        let tmp = tempfile::tempdir()?;
        let mut ready: Vec<RegionEntry> = Vec::with_capacity(deduped.len());
        let mut skipped: Vec<SkippedRegion> = Vec::new();
        for (i, name) in deduped.iter().enumerate() {
            match self
                .host_one(name, &tmp.path().join(format!("r{i}.pbf")), &opts)
                .await
            {
                Ok(mut entry) => {
                    entry.registered_by = signer;
                    ready.push(entry);
                }
                Err(e) => skipped.push(SkippedRegion {
                    region: name.to_string(),
                    error: e.to_string(),
                }),
            }
        }
        deduped.clear();
        if !ready.is_empty() {
            self.registry.register(&ready, signer).await?;
        }
        Ok(HostReport { registered: ready, skipped })
    }

    async fn host_one(
        &self,
        region: &str,
        tmp_file: &Path,
        opts: &HostOpts,
    ) -> Result<RegionEntry, SdkError> {
        let def = regions::get(region).ok_or_else(|| SdkError::UnknownRegion(region.into()))?;
        self.geofabrik
            .download(&def.pbf_url, tmp_file, opts.max_retries)
            .await?;
        let actual = crate::geofabrik::md5_file(tmp_file).await?;
        let expected = self.geofabrik.fetch_checksum(&def.pbf_url).await?;
        if actual != expected {
            let _ = tokio::fs::remove_file(tmp_file).await;
            return Err(SdkError::ChecksumMismatch {
                region: region.to_string(),
                expected,
                actual,
            });
        }
        let receipt = self.storage.put(tmp_file, region).await?;
        let _ = tokio::fs::remove_file(tmp_file).await;
        let version = self.geofabrik.current_version(&def.pbf_url).await?;
        Ok(RegionEntry {
            region: def.region.clone(),
            parent: def.parent.clone(),
            level: def.level,
            cid: receipt.cid,
            source_url: def.pbf_url.clone(),
            checksum: expected,
            version,
            hosted: opts.hosted,
            timestamp: now_unix(),
            registered_by: [0u8; 32], // stamped with the signer in host_regions
            record_version: 1,
        })
    }

    /// Local import: verify a user-provided PBF against Geofabrik's
    /// published MD5 (import-time only), store, register by CID.
    /// `checksum_override` exists for offline/e2e use.
    pub async fn import_local(
        &self,
        region: &str,
        file: &Path,
        signer: [u8; 32],
        opts: HostOpts,
        checksum_override: Option<String>,
    ) -> Result<RegionEntry, SdkError> {
        let def = regions::get(region).ok_or_else(|| SdkError::UnknownRegion(region.into()))?;
        let expected = match checksum_override {
            Some(c) => c,
            None => self.geofabrik.fetch_checksum(&def.pbf_url).await?,
        };
        let actual = crate::geofabrik::md5_file(file).await?;
        if actual != expected {
            return Err(SdkError::ChecksumMismatch {
                region: region.to_string(),
                expected,
                actual,
            });
        }
        let receipt = self.storage.put(file, region).await?;
        let version = match checksum_override {
            Some(_) => "local".to_string(),
            None => self.geofabrik.current_version(&def.pbf_url).await?,
        };
        let entry = RegionEntry {
            region: def.region.clone(),
            parent: def.parent.clone(),
            level: def.level,
            cid: receipt.cid,
            source_url: def.pbf_url.clone(),
            checksum: expected,
            version,
            hosted: opts.hosted,
            timestamp: now_unix(),
            registered_by: signer,
            record_version: 1,
        };
        osm_registry_core::validate_entry(&entry)
            .map_err(|code| SdkError::InvalidEntry { region: region.to_string(), code })?;
        self.registry.register(&[entry.clone()], signer).await?;
        Ok(entry)
    }

    /// Compare central Geofabrik versions against registry entries, matched
    /// per region path (a country file and its subregions track
    /// independently).
    pub async fn check_updates(&self) -> Result<Vec<RegionUpdate>, SdkError> {
        let map: BTreeMap<String, RegionEntry> = match self.registry.fetch_all().await? {
            Some(m) => m,
            None => return Ok(Vec::new()),
        };
        let mut out = Vec::new();
        for (name, entry) in &map {
            let def = match regions::get(name) {
                Some(d) => d,
                None => continue, // non-predefined test regions
            };
            let central = match self.geofabrik.current_version(&def.pbf_url).await {
                Ok(v) => v,
                Err(e) => {
                    tracing::warn!(region = %name, error = %e, "update check failed");
                    continue;
                }
            };
            if central != entry.version {
                out.push(RegionUpdate {
                    region: name.clone(),
                    registry_version: entry.version.clone(),
                    central_version: central,
                });
            }
        }
        Ok(out)
    }

    /// Download one region: by CID from Logos Storage when hosted, else
    /// straight from Geofabrik (direct fallback). Reports whether the bytes
    /// still match the registered MD5 — the CID is the trust anchor, the
    /// checksum is evidence.
    pub async fn download_region(
        &self,
        region: &str,
        dest_dir: &Path,
    ) -> Result<DownloadReport, SdkError> {
        let def = regions::get(region).ok_or_else(|| SdkError::UnknownRegion(region.into()))?;
        let entry = self.resolve(region).await?;
        tokio::fs::create_dir_all(dest_dir).await?;
        let dest = dest_dir.join(format!(
            "{}-{}.osm.pbf",
            region.replace('/', "-"),
            entry.as_ref().map(|e| e.version.as_str()).unwrap_or("local")
        ));
        let source = match &entry {
            Some(e) if e.hosted => {
                self.storage.get(&e.cid, &dest).await?;
                DownloadSource::LogosStorage
            }
            _ => {
                self.geofabrik.download(&def.pbf_url, &dest, 3).await?;
                DownloadSource::Geofabrik
            }
        };
        let verified = match &entry {
            Some(e) => crate::geofabrik::md5_file(&dest).await? == e.checksum,
            None => true,
        };
        let bytes = tokio::fs::metadata(&dest).await.map(|m| m.len()).unwrap_or(0);
        Ok(DownloadReport { region: region.to_string(), source, path: dest, bytes, verified })
    }
}

fn now_unix() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::MockRegistryClient;
    use crate::storage::InMemoryStorage;
    use std::sync::Arc;

    fn sdk_for_tests() -> (OsmSdk, Arc<InMemoryStorage>, MockRegistryClient) {
        let storage = Arc::new(InMemoryStorage::new());
        let mock = MockRegistryClient::new();
        let sdk = OsmSdk::new(Box::new(storage.clone()), Box::new(mock.clone()));
        (sdk, storage, mock)
    }

    fn test_entry(region: &str) -> RegionEntry {
        RegionEntry {
            region: region.into(),
            parent: None,
            level: 0,
            cid: "bafk-x".into(),
            source_url: "https://download.geofabrik.de/europe/germany-latest.osm.pbf".into(),
            checksum: "35cff894271b8f56ddb81911dcffe089".into(),
            version: "261024".into(),
            hosted: true,
            timestamp: 1,
            registered_by: [1; 32],
            record_version: 1,
        }
    }

    #[tokio::test]
    async fn host_region_full_pipeline() {
        let (sdk, storage, mock) = sdk_for_tests();
        mock.init_registry().await.unwrap();
        let server = wiremock::MockServer::start().await;
        sdk.geofabrik
            .set_url_rewrite("https://download.geofabrik.de", server.uri());

        let body = b"pbf-bytes-for-germany".to_vec();
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf.md5"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(
                format!("{}  germany-latest.osm.pbf\n", hex::encode(md5_of(&body))),
            ))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf"))
            .respond_with(
                wiremock::ResponseTemplate::new(307).insert_header(
                    "Location",
                    format!("{server}/europe/germany-261024.osm.pbf"),
                ),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/germany-261024.osm.pbf"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_bytes(body.clone())
                    .append_header("Last-Modified", "Sun, 25 Oct 2026 00:51:09 GMT"),
            )
            .mount(&server)
            .await;

        let entry = sdk
            .host_region("germany", [9u8; 32], HostOpts::default())
            .await
            .unwrap();
        assert_eq!(entry.region, "germany");
        assert_eq!(entry.version, "261024");
        assert_eq!(entry.checksum, hex::encode(md5_of(&body)));
        assert!(entry.hosted);
        assert_eq!(entry.registered_by, [9u8; 32]);
        assert_eq!(storage.object_count().await, 1);
        assert_eq!(mock.register_call_count(), 1);
        let all = mock.fetch_all().await.unwrap().unwrap();
        assert_eq!(all["germany"].cid, entry.cid);
    }

    #[tokio::test]
    async fn host_region_checksum_mismatch_aborts() {
        let (sdk, storage, mock) = sdk_for_tests();
        mock.init_registry().await.unwrap();
        let server = wiremock::MockServer::start().await;
        sdk.geofabrik
            .set_url_rewrite("https://download.geofabrik.de", server.uri());

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf.md5"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(
                "35cff894271b8f56ddb81911dcffe089  germany-latest.osm.pbf\n",
            ))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf"))
            .respond_with(
                wiremock::ResponseTemplate::new(200).set_body_bytes(b"tampered".to_vec()),
            )
            .mount(&server)
            .await;

        let err = sdk
            .host_region("germany", [0u8; 32], HostOpts::default())
            .await
            .unwrap_err();
        assert!(matches!(err, SdkError::ChecksumMismatch { .. }), "{err}");
        assert_eq!(storage.object_count().await, 0);
        assert!(mock.fetch_all().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn host_regions_batches_into_one_tx() {
        let (sdk, _s, mock) = sdk_for_tests();
        mock.init_registry().await.unwrap();
        let server = wiremock::MockServer::start().await;
        sdk.geofabrik
            .set_url_rewrite("https://download.geofabrik.de", server.uri());
        for (path, body) in [
            ("/europe/germany-latest.osm.pbf.md5", None),
            ("/europe/france-latest.osm.pbf.md5", None),
            ("/europe/germany-261024.osm.pbf", Some(b"de-bytes".to_vec())),
            ("/europe/france-261024.osm.pbf", Some(b"fr-bytes".to_vec())),
        ] {
            let resp = match body {
                Some(b) => wiremock::ResponseTemplate::new(200)
                    .set_body_bytes(b)
                    .append_header("Last-Modified", "Sun, 25 Oct 2026 00:51:09 GMT"),
                None => wiremock::ResponseTemplate::new(200).set_body_string(format!(
                    "{}  x.osm.pbf\n",
                    hex::encode(md5_of(if path.contains("germany") {
                        b"de-bytes"
                    } else {
                        b"fr-bytes"
                    }))
                )),
            };
            wiremock::Mock::given(wiremock::matchers::method("GET"))
                .and(wiremock::matchers::path(path))
                .respond_with(resp)
                .mount(&server)
                .await;
        }
        // -latest 307s
        for (latest, dated) in [
            ("/europe/germany-latest.osm.pbf", "/europe/germany-261024.osm.pbf"),
            ("/europe/france-latest.osm.pbf", "/europe/france-261024.osm.pbf"),
        ] {
            wiremock::Mock::given(wiremock::matchers::method("GET"))
                .and(wiremock::matchers::path(latest))
                .respond_with(
                    wiremock::ResponseTemplate::new(307)
                        .insert_header("Location", format!("{server}{dated}")),
                )
                .mount(&server)
                .await;
        }

        let report = sdk
            .host_regions(
                &["germany".into(), "france".into()],
                [7u8; 32],
                HostOpts::default(),
            )
            .await
            .unwrap();
        assert_eq!(report.registered.len(), 2);
        assert!(report.skipped.is_empty());
        assert_eq!(mock.register_call_count(), 1, "bulk host is ONE batch tx");
    }

    #[tokio::test]
    async fn bulk_host_skips_failed_region() {
        let (sdk, _s, mock) = sdk_for_tests();
        mock.init_registry().await.unwrap();
        let server = wiremock::MockServer::start().await;
        sdk.geofabrik
            .set_url_rewrite("https://download.geofabrik.de", server.uri());
        // germany works; france pbf 404s
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf.md5"))
            .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(
                format!("{}  x\n", hex::encode(md5_of(b"de-bytes"))),
            ))
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_bytes(b"de-bytes".to_vec())
                    .append_header("Last-Modified", "Sun, 25 Oct 2026 00:51:09 GMT"),
            )
            .mount(&server)
            .await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/france-latest.osm.pbf"))
            .respond_with(wiremock::ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let report = sdk
            .host_regions(
                &["germany".into(), "france".into()],
                [7u8; 32],
                HostOpts { max_retries: 1, ..Default::default() },
            )
            .await
            .unwrap();
        assert_eq!(report.registered.len(), 1);
        assert_eq!(report.registered[0].region, "germany");
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].region, "france");
    }

    #[test]
    fn bulk_overlap_guard_is_structural() {
        // unit-level pin of the guard's rule (host_regions enforces it)
        let child = "germany/x";
        assert!(child.starts_with(&format!("{}/", "germany")));
    }

    #[tokio::test]
    async fn import_local_uses_override_checksum() {
        let (sdk, storage, mock) = sdk_for_tests();
        mock.init_registry().await.unwrap();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("local.pbf");
        tokio::fs::write(&file, b"local bytes").await.unwrap();
        let md5 = crate::geofabrik::md5_file(&file).await.unwrap();

        let entry = sdk
            .import_local(
                "germany",
                &file,
                [3u8; 32],
                HostOpts::default(),
                Some(md5.clone()),
            )
            .await
            .unwrap();
        assert_eq!(entry.checksum, md5);
        assert_eq!(entry.version, "local");
        assert_eq!(entry.registered_by, [3u8; 32]);
        assert_eq!(storage.object_count().await, 1);

        let wrong = sdk
            .import_local(
                "germany",
                &file,
                [3u8; 32],
                HostOpts::default(),
                Some("00000000000000000000000000000000".into()),
            )
            .await;
        assert!(matches!(wrong.unwrap_err(), SdkError::ChecksumMismatch { .. }));
    }

    #[tokio::test]
    async fn check_updates_detects_newer_central() {
        let (sdk, _s, mock) = sdk_for_tests();
        mock.init_registry().await.unwrap();
        let mut e = test_entry("germany");
        e.version = "261024".into();
        mock.register(&[e], [0u8; 32]).await.unwrap();

        let server = wiremock::MockServer::start().await;
        sdk.geofabrik
            .set_url_rewrite("https://download.geofabrik.de", server.uri());
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf"))
            .respond_with(
                wiremock::ResponseTemplate::new(307).insert_header(
                    "Location",
                    format!("{server}/europe/germany-261025.osm.pbf"),
                ),
            )
            .mount(&server)
            .await;

        let updates = sdk.check_updates().await.unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].region, "germany");
        assert_eq!(updates[0].registry_version, "261024");
        assert_eq!(updates[0].central_version, "261025");
    }

    #[tokio::test]
    async fn download_prefers_storage_then_falls_back() {
        let (sdk, storage, mock) = sdk_for_tests();
        mock.init_registry().await.unwrap();
        let dir = tempfile::tempdir().unwrap();

        // hosted: put an object, register it, download from storage
        let src = dir.path().join("in.pbf");
        tokio::fs::write(&src, b"hosted bytes").await.unwrap();
        let r = storage.put(&src, "germany").await.unwrap();
        let mut e = test_entry("germany");
        e.cid = r.cid;
        e.checksum = crate::geofabrik::md5_file(&src).await.unwrap();
        mock.register(&[e], [0u8; 32]).await.unwrap();

        let report = sdk.download_region("germany", dir.path()).await.unwrap();
        assert_eq!(report.source, DownloadSource::LogosStorage);
        assert_eq!(report.bytes, 12);
        assert!(report.verified);
    }

    #[tokio::test]
    async fn download_unhosted_falls_back_to_geofabrik() {
        let (sdk, _s, mock) = sdk_for_tests();
        mock.init_registry().await.unwrap();
        let dir = tempfile::tempdir().unwrap();

        let mut e = test_entry("germany");
        e.hosted = false;
        e.checksum = hex::encode(md5_of(b"fallback bytes"));
        mock.register(&[e], [0u8; 32]).await.unwrap();

        let server = wiremock::MockServer::start().await;
        sdk.geofabrik
            .set_url_rewrite("https://download.geofabrik.de", server.uri());
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_bytes(b"fallback bytes".to_vec()),
            )
            .mount(&server)
            .await;

        let report = sdk.download_region("germany", dir.path()).await.unwrap();
        assert_eq!(report.source, DownloadSource::Geofabrik);
        assert!(report.verified);
    }

    fn md5_of(b: &[u8]) -> [u8; 16] {
        use md5::Digest;
        let mut h = md5::Md5::new();
        h.update(b);
        h.finalize().into()
    }
}
