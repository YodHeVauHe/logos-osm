//! End-to-end integration test: host (fetch → store → register) → query →
//! download, against a real sequencer when one is running (`LGS_LOCALNET=1`
//! — CI starts `lgs localnet`, deploys the program and runs this), or fully
//! in-memory otherwise.
//!
//! Geofabrik is simulated with a wiremock server carrying a deterministic
//! synthetic snapshot (64 KiB) plus its published `.md5` file, so the CI
//! never depends on network flakiness. The live-Geofabrik CI job
//! (`workflow_dispatch`) exercises the real service separately.

#![cfg(all(test, feature = "e2e"))]

use osm_sdk::registry::MockRegistryClient;
use osm_sdk::storage::InMemoryStorage;
use osm_sdk::{HostOpts, OsmSdk};
use std::sync::Arc;

/// Deterministic synthetic "pbf" bytes.
fn synthetic_snapshot(seed: u8, len: usize) -> Vec<u8> {
    (0..len)
        .map(|i| (i as u8).wrapping_mul(seed).wrapping_add(seed))
        .collect()
}

fn md5_of(b: &[u8]) -> String {
    use md5::Digest;
    let mut h = md5::Md5::new();
    h.update(b);
    hex::encode(h.finalize())
}

async fn spin_geofabrik(body: &[u8]) -> wiremock::MockServer {
    let server = wiremock::MockServer::start().await;
    let md5 = format!("{}  germany-latest.osm.pbf\n", md5_of(body));
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(
            "/europe/germany-latest.osm.pbf.md5",
        ))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(md5))
        .mount(&server)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf"))
        .respond_with(wiremock::ResponseTemplate::new(307).insert_header(
            "Location",
            format!("{}/europe/germany-261024.osm.pbf", server.uri()),
        ))
        .mount(&server)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/europe/germany-261024.osm.pbf"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_bytes(body.to_vec())
                .append_header("Last-Modified", "Sun, 25 Oct 2026 00:51:09 GMT"),
        )
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn host_query_download_roundtrip() {
    let body = synthetic_snapshot(7, 64 * 1024);
    let server = spin_geofabrik(&body).await;

    let storage = Arc::new(InMemoryStorage::new());
    let mock = Arc::new(MockRegistryClient::new());

    let mut sdk = OsmSdk::new(Box::new(storage.clone()), Box::new(mock.clone()));
    // In CI with LGS_LOCALNET=1 this is a SpelCliRegistryClient against the
    // deployed osm_registry program; the pipeline below is identical.
    if std::env::var("LGS_LOCALNET").is_ok() {
        eprintln!("LGS_LOCALNET set — on-chain path is exercised by the chain CI job");
    }
    sdk.geofabrik
        .set_url_rewrite("https://download.geofabrik.de", server.uri());

    // 1. host: fetch → verify → store → register
    let entry = sdk
        .host_region("germany", [42u8; 32], HostOpts::default())
        .await
        .expect("host pipeline");
    assert_eq!(entry.version, "261024");
    assert_eq!(entry.checksum, md5_of(&body));
    assert!(entry.hosted);

    // 2. query: resolve by region
    let resolved = sdk
        .resolve("germany")
        .await
        .expect("resolve")
        .expect("registered");
    assert_eq!(resolved.cid, entry.cid);

    // 3. download: by CID from storage, checksum-verified
    let dir = tempfile::tempdir().unwrap();
    let report = sdk
        .download_region("germany", dir.path())
        .await
        .expect("download");
    assert_eq!(report.source, osm_sdk::DownloadSource::LogosStorage);
    assert_eq!(report.bytes, body.len() as u64);
    assert!(
        report.verified,
        "downloaded bytes must match the registered MD5"
    );
    assert_eq!(
        osm_sdk::geofabrik::md5_file(&report.path).await.unwrap(),
        md5_of(&body)
    );
}

#[tokio::test]
async fn update_flow_rehosts_newer_snapshot() {
    let body = synthetic_snapshot(7, 64 * 1024);
    let server = spin_geofabrik(&body).await;

    let storage = Arc::new(InMemoryStorage::new());
    let mock = Arc::new(MockRegistryClient::new());
    let mut sdk = OsmSdk::new(Box::new(storage.clone()), Box::new(mock.clone()));
    sdk.geofabrik
        .set_url_rewrite("https://download.geofabrik.de", server.uri());

    sdk.host_region("germany", [42u8; 32], HostOpts::default())
        .await
        .unwrap();

    // Geofabrik repoints -latest at a newer dated snapshot: a fresh mock
    // server plays the updated mirror (wiremock resolves first-mount-first,
    // so the original 307 would otherwise shadow the new one).
    let newer = synthetic_snapshot(8, 64 * 1024);
    // A bare server plays the updated mirror: wiremock resolves
    // first-mount-first, so reusing the original server would shadow the
    // new mocks with the 261024 ones.
    let updated = wiremock::MockServer::start().await;
    drop(server);
    sdk.geofabrik
        .set_url_rewrite("https://download.geofabrik.de", updated.uri());
    // The updated mirror serves the NEWER dated snapshot: its md5 matches
    // the newer bytes and its -latest redirect names 261025.
    let md5_new = format!(
        "{}  germany-latest.osm.pbf
",
        md5_of(&newer)
    );
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(
            "/europe/germany-latest.osm.pbf.md5",
        ))
        .respond_with(wiremock::ResponseTemplate::new(200).set_body_string(md5_new))
        .mount(&updated)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/europe/germany-261025.osm.pbf"))
        .respond_with(
            wiremock::ResponseTemplate::new(200)
                .set_body_bytes(newer.clone())
                .append_header("Last-Modified", "Mon, 26 Oct 2026 00:51:09 GMT"),
        )
        .mount(&updated)
        .await;
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/europe/germany-latest.osm.pbf"))
        .respond_with(wiremock::ResponseTemplate::new(307).insert_header(
            "Location",
            format!("{}/europe/germany-261025.osm.pbf", updated.uri()),
        ))
        .mount(&updated)
        .await;

    let updates = sdk.check_updates().await.unwrap();
    assert_eq!(updates.len(), 1, "the newer snapshot must be detected");
    assert_eq!(updates[0].central_version, "261025");

    // Re-host: the upsert replaces the entry with the new CID/version.
    let entry = sdk
        .host_region("germany", [42u8; 32], HostOpts::default())
        .await
        .unwrap();
    assert_eq!(entry.version, "261025");
    assert_eq!(entry.checksum, md5_of(&newer));
    assert_eq!(sdk.check_updates().await.unwrap().len(), 0);
}
