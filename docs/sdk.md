# OSM registry SDK

`osm_sdk` is the single Rust crate behind the CLI, the C FFI, and the
Basecamp module. It has two faces:

- **Consumer face** — discover regions, resolve a region to its CID and
  metadata, query by region/parent/CID, and download. This is the subset
  other Basecamp apps need; it is what the published `osm` module exposes.
- **Distribution face** — host (fetch → verify → store → register), bulk
  host in one batch transaction, local import with Geofabrik-MD5
  verification, and update checks.

## The region model

`regions::predefined()` returns the frozen, non-overlapping 72-region set
(embedded from `regions.json`). Each `RegionDef` carries the Geofabrik
index id, the canonical `*-latest.osm.pbf` URL (parent-chain based —
`india/central-zone` downloads from `.../asia/india/central-zone-...`), and
the level/parent encoding that round-trips into an on-chain `RegionEntry`.

Two frozen set members have no standalone Geofabrik extract:

| registry key | Geofabrik extract used |
|---|---|
| `ireland` | `ireland-and-northern-ireland` |
| `malaysia` | `malaysia-singapore-brunei` |

Registry keys stay exactly as frozen in the prize document; only the
URL/checksum resolution maps through the combined extract.

## Worked example: resolve a region to its CID and metadata

The integration path most Basecamp consumers take:

```rust
use osm_sdk::registry::{LezWalletRegistryClient, MockRegistryClient, RegistryClient};
use osm_sdk::storage::InMemoryStorage;                      // or LogoscoreStorage
use osm_sdk::OsmSdk;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = OsmSdk::new(
        Box::new(InMemoryStorage::new()),     // downloads don't need storage
        Box::new(LezWalletRegistryClient {
            cli_bin: "./examples/target/debug/osm_registry_cli".into(),
            program_id: "<deployed-v0.3.1-program-account-id>".into(),
            rpc_url: "http://127.0.0.1:3040".into(),
            wallet_home: "/tmp/lez-wallet".into(),
            signer_account_id: "<fee-payer-account-id>".into(),
        }),
    );

    // 1. Is `us/california` registered? What are its CID and version?
    match sdk.resolve("us/california").await? {
        Some(entry) => {
            println!("region:   {}", entry.region);
            println!("cid:      {}", entry.cid);          // Logos Storage CID
            println!("version:  {}", entry.version);     // Geofabrik YYMMDD
            println!("checksum: {}", entry.checksum);    // Geofabrik MD5
            println!("hosted:   {}", entry.hosted);
            if entry.hosted {
                // 2. Download by CID — integrity is guaranteed by the
                //    content address; no Geofabrik round-trip required.
                let report = sdk.download_region("us/california", "./downloads".as_ref()).await?;
                println!("{} bytes from {:?}, verified={}", report.bytes, report.source, report.verified);
            }
        }
        None => println!("us/california is not hosted yet"),
    }

    // Query facets
    let zones = sdk.query_by_parent("india").await?;       // all 6 India zones
    let by_cid = sdk.query_by_cid(&zones[0].cid).await?;   // CID → region(s)

    // Full set with fresh Geofabrik URLs (network)
    // sdk.discover_refreshed(&fetch_index_json())?;

    Ok(())
}
```

`resolve` on an unregistered predefined region returns `Ok(None)`; asking
for a region outside the frozen set is `Err(SdkError::UnknownRegion)`.

## Basecamp module consumer

[`examples/basecamp-consumer`](../examples/basecamp-consumer) is a minimal
universal Logos module that declares `osm` as a required module dependency.
Its generated typed client calls `modules().osm.resolve(region)` to return the
registry entry JSON and `modules().osm.downloadRegion(region, destDir)` to
download the hosted snapshot (or use the module's Geofabrik fallback). The
consumer does not link the Rust SDK or depend on `osm-app`.

Build the installable consumer package from the repository root with:

```sh
nix build .#osm-consumer-example
```

The root flake passes its `osm` module derivation to `logos-module-builder`,
which generates the dependency wrapper and bundles the OSM module and its
transitive dependencies into the consumer package.

## Hosting (distribution face)

```rust
// One region: download → verify Geofabrik MD5 → store → register_region.
let entry = sdk.host_region("germany", signer, HostOpts::default()).await?;

// Bulk: N regions verified+stored, then ONE register_batch transaction.
let report = sdk.host_regions(
    &["germany".into(), "us/california".into(), "india/central-zone".into()],
    signer,
    HostOpts::default(),
).await?;   // report.skipped carries per-region failures

// Local import: verify a provided PBF against Geofabrik's published MD5,
// store, register by CID.
let entry = sdk.import_local("kenya", "./kenya-latest.osm.pbf".as_ref(), signer,
                             HostOpts::default(), None).await?;

// Updates: per-region central-version comparison (matched on region path).
for u in sdk.check_updates().await? {
    println!("{}: registry={} geofabrik={}", u.region, u.registry_version, u.central_version);
}
```

## Backends

| Concern | Production | Tests / dev |
|---|---|---|
| Storage | `LogoscoreStorage` (drives the headless `logoscore` binary with `storage_module`) | `InMemoryStorage` |
| Registry client | `LezWalletRegistryClient` (LEZ v0.3.1 wallet/RPC CLI for transactions and public account reads) | `MockRegistryClient` (feature `test-util`) |
| Geofabrik | real service | `set_url_rewrite(base)` → wiremock or a mirror |

Retries: downloads and storage uploads retry with exponential back-off
(1s, 2s, 4s … capped 30s) and surface `SdkError::RetriesExhausted` with the
full context after the last attempt. Checksum mismatches fail fast with
`ChecksumMismatch { expected, actual }` and never register anything.

`LEE_WALLET_HOME_DIR` selects the v0.3.1 wallet home. Set `OSM_RPC_URL` (or pass the CLI's `--rpc-url`) to select a sequencer. The deployed program account ID is the program-loader header account, distinct from the image ID printed by `make build`. Registry bytes are fetched from `account.data.shard(program_account_id)` and Borsh-decoded as `osm_registry_core::Registry`.
