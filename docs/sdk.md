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
use osm_sdk::storage::InMemoryStorage;                      // or LogosctlStorage
use osm_sdk::OsmSdk;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk = OsmSdk::new(
        Box::new(InMemoryStorage::new()),     // downloads don't need storage
        Box::new(LezWalletRegistryClient {
            cli_bin: "./examples/target/debug/osm_registry_cli".into(),
            program_id: "<deployed-v0.4-program-account-id>".into(),
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

### Embedding example: a Basecamp UI app on top of the consumer

[`examples/basecamp-consumer-app`](../examples/basecamp-consumer-app) is a
minimal `mkLogosQmlModule` UI app (`osm-consumer-app`) whose only module
dependency is `osm_consumer_example`. It shows how a third-party Basecamp app
uses OSM through another module rather than linking anything itself:

```
osm-consumer-app (QML + backend) → osm_consumer_example → osm → LEZ registry / Logos Storage
```

Its backend (`src/osm_consumer_app_backend.cpp`) calls three consumer methods
with `invokeRemoteMethod` and an explicit timeout:

| Consumer method | Purpose |
|---|---|
| `configure(configJson)` | forwards `program_id` / `cli_bin` to `osm.setConfig` (read from `OSM_PROGRAM_ID` / `OSM_REGISTRY_CLI`) |
| `resolveRegion(region)` | returns the on-chain registry entry JSON |
| `downloadRegion(region, destDir)` | downloads the snapshot and returns `{path, verified, source, bytes}` |

The QML view (`src/qml/Main.qml`) has a region field (default `china/henan`),
**Resolve** and **Download** buttons, and shows the entry and the verified path.
Downloads go to the user's Downloads folder.

Build both installable packages from the repository root:

```sh
nix build .#osm-consumer-example-lgx-portable   # consumer core module (.lgx)
nix build .#osm-consumer-app-lgx-portable       # UI app (.lgx)
```

Verified in portable Basecamp (66505d5, Linux x86_64) on 2026-11-02 with the
v0.3 testnet registry: the app resolved `china/henan` and downloaded the
50,468,082-byte `china-henan-261031.osm.pbf` from Logos Storage, with its MD5
(`54c5fbf47dc51588de2ffb0a0a94a771`) matching the registry entry.

Both packages are published in the module catalog as
`osm_consumer_example` 1.0.2 and `osm-consumer-app` 1.0.0, with
`linux-amd64` and `darwin-arm64` variants (see [`catalog.md`](catalog.md)),
so `lgpd install osm-consumer-app` pulls the app, the consumer and `osm`. The
check above was repeated with the catalog `.lgx` files; screenshots are in
[`basecamp-check.md`](basecamp-check.md#consumer-app-from-the-catalog-sdk-embedding-example).

### Timeouts and storage when calling `downloadRegion`

`downloadRegion` is a synchronous call that returns once the snapshot is on
disk and verified, so it can take much longer than an ordinary RPC:

- The generated client's default RPC timeout is 20 s. The consumer passes an
  explicit timeout (`modules().osm.downloadRegion(region, destDir, &err,
  15 * 60 * 1000)`) and turns a failed call into an
  `{"ok":false,"error":...}` object instead of an empty string. Before this,
  the consumer gave up after 20 s and `logosctl call` reported `RPC_FAILED`
  while `osm` was still downloading.
- `logosctl call` itself (logosctl 0.3.2) waits 20 s for the method it calls
  and has no option to raise that, so a CLI call into the consumer or into
  `osm` fails with `timed out after 20000ms` for any download that takes longer.
  This limit is upstream. Basecamp apps and other modules call with their own
  timeouts and are not limited by it. `osm` keeps downloading after the caller
  gives up, and later calls to `osm` wait until that download ends.
- In a `logosctl` session the daemon starts each module host with
  `LOGOSCTL_CONFIG_DIR` set to that session's directory. So, unless told
  otherwise, `osm` fetches through the session's own `storage_module`. A fresh
  node there may not reach the node holding the CID for minutes. Point `osm` at
  a storage node that is connected to the network with
  `setConfig` (`"logosctl_bin"` and `"storage_config_dir"`):

  ```sh
  logosctl call osm setConfig '{"program_id":"<id>","cli_bin":"<osm_registry_cli>","logosctl_bin":"<logosctl>","storage_config_dir":"<dir of the storage node>"}'
  logosctl call osm_consumer_example downloadRegion china/henan /path/to/dir
  ```

- `destDir` may be a plain path or a local `file://` URL. Both `osm` and
  `osm-app` convert the URL to a path. Before 1.0.2, a `file://` URL created a
  relative `file:` directory in the host's working directory.

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
| Storage | `LogosctlStorage` (calls the current `storage_module` API through `logosctl`) | `InMemoryStorage` |
| Registry client | `LezWalletRegistryClient` (LEZ v0.4 wallet/RPC CLI for transactions and public account reads) | `MockRegistryClient` (feature `test-util`) |
| Geofabrik | real service | `set_url_rewrite(base)` → wiremock or a mirror |

Configure the production storage backend with `logosctl_bin` and
`storage_config_dir` in `osm.toml`, or `LOGOSCTL_BIN` and
`LOGOSCTL_CONFIG_DIR`. `LogosctlStorage` talks to a running daemon through the
current `storage_module` API (`uploadUrl` and `downloadToUrl`) and waits for
matching completion events. The ignored live test
`storage::tests::live_logosctl_roundtrip` can be run with those environment
variables plus `STORAGE_ROUNDTRIP_FILE` and `STORAGE_ROUNDTRIP_DEST`. A local
roundtrip was verified with logosctl 0.3.2 and storage_module 3.0.2 (libstorage
v0.5.3): the 3,507,206-byte Andorra extract returned CID
`zDvZRwzkxTZxVLVzFakT2w2vFqF6AXUgBQtWiBQinTqJsb3NyLrU`; source and downloaded
files had MD5 `fb146ceabccea8a2c9e2bf20618d0bc5` and SHA-256
`7f4f43ac2c29226d61b2d043d5c6a998d3112468717203b838a16b504108498c`. This
exercises one local node and does not demonstrate transfer between nodes.

Retries: downloads and storage uploads retry with exponential back-off
(1s, 2s, 4s … capped 30s) and surface `SdkError::RetriesExhausted` with the
full context after the last attempt. Checksum mismatches fail fast with
`ChecksumMismatch { expected, actual }` and never register anything.

`LEE_WALLET_HOME_DIR` selects the v0.4 wallet home (set `LEE_WALLET_PASSWORD` if its storage is encrypted). Set `OSM_RPC_URL` (or pass the CLI's `--rpc-url`) to select a sequencer. The deployed program account ID is the program-loader header account, distinct from the image ID printed by `make build`. Registry bytes are fetched from `account.data.actor_state(program_account_id)` and Borsh-decoded as `osm_registry_core::Registry`.
