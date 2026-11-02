# Basecamp check: osm-app on Linux against testnet v0.3

This is how `osm-app` was checked by hand in Logos Basecamp on 2026-11-02:
Linux x86_64, the portable Basecamp build, and the LEZ v0.3.1 testnet.

## Result

- Basecamp loads `storage_module` 3.0.2, `osm` and `osm-app_backend`, and
  opens the `osm-app` tab with no `appBridge` errors.
- The app lists the predefined regions (from `osm.discover`).
- With `OSM_PROGRAM_ID` and `OSM_REGISTRY_CLI` set, the app reads the
  on-chain registry. The 25 regions hosted on testnet v0.3 show
  `hosted · 261031`. The same 25 regions, all at version `261031`, come back
  from `osm_registry_cli get` run outside the app.
- Not covered in the first pass: the **Host selected** button, which needs a
  funded wallet; macOS. **Download** failed in the 1.0.1 recheck and passes
  in the 1.0.2 recheck, both below.

Evidence (in [`evidence/basecamp/`](evidence/basecamp/)):

| File | Shows |
| --- | --- |
| `basecamp-portable-osm-app-loaded-no-bridge.png` | osm-app tab loaded through the module backend, before the region list |
| `basecamp-portable-osm-app-discover.png` | predefined regions listed from `osm.discover` |
| `basecamp-osm-app-registry-lookup-testnet-v0.3.png` | registry lookup against testnet v0.3, hosted regions marked |
| `basecamp-osm-app-kenya-hosted-testnet-v0.3.png` | e.g. kenya, egypt, pakistan shown `hosted · 261031` |
| `osm_registry_cli-get-testnet-v0.3.json` | `osm_registry_cli get` output for the same program |

## Recheck with the catalog 1.0.1 build

Rerun on 2026-11-02 (13:00 EAT) with the `osm` 1.0.1 and `osm-app` 1.0.1
bundles from the [module catalog](catalog.md). The installed `osm_plugin.so`
and `osm-app_backend_plugin.so` are byte-identical to the ones unpacked from
the catalog `.lgx` files, whose sha256 match `index.json`.

| Check | Result |
| --- | --- |
| Load | Pass. Basecamp loads `storage_module` 3.0.2, `osm` 1.0.1 and `osm-app_backend`, and opens the `osm-app` tab. |
| Hosted regions from testnet v0.3 | Pass. After **Refresh** finishes, the screenshots show 21 of the 25 registered regions as `hosted · 261031` (11 countries, all 10 subregions). The other four (bulgaria, hungary, romania, south-korea) are higher in the list and were not captured. Every row seen agrees with `osm_registry_cli get`: registered regions show hosted, the rest show `not hosted`. Screenshots taken before the refresh finished show every row as `not hosted`. |
| **Download** (`china/henan`, 50.5 MB) | **Fail.** The app shows `downloading china/henan…`, then `invalid module JSON: unterminated object`. No file is written. |
| **Host selected** | Not run. `osm_host` registers the region on chain, which signs a transaction and spends deployer tokens. |
| Consumer example | Not runnable in Basecamp: `osm_consumer_example` is a headless core module, and Basecamp only loads core modules as dependencies of a UI app. Run in a `logosctl` session instead (below). |

### Why Download fails

Two bugs, both reproduced outside the app with the same 1.0.1
`libosm_ffi.so` and the same environment as the `osm` module host:

1. `Main.qml` passes `StandardPaths.writableLocation(StandardPaths.DownloadLocation)`
   as `destDir`. In QML this is a URL (`file:///home/box/Downloads`), not a
   path. The FFI treats it as a relative path, creates
   `./file:/home/box/Downloads` under the working directory, and Storage
   fails with `cannot open: file:///…/china-henan-261031.osm.pbf`.
2. `OsmImpl::callFfi` builds the error reply as
   `{"ok":false,"error":"<lastError>"}` without escaping. The Storage error
   contains quotes, so the app cannot parse the reply and shows
   `invalid module JSON` instead of the real error.

With a plain directory path, the same `osm_download` call returns
`{"source":"logos_storage","bytes":50468082,"verified":true}` in about 3 s,
and the file's MD5 (`54c5fbf47dc51588de2ffb0a0a94a771`) matches the
registry checksum. The data comes from the box's own Storage node, so this
does not show retrieval from another node.

### Consumer example in logosctl

`osm` 1.0.1 and `osm_consumer_example` loaded in a `logosctl` 0.3.2 session.
The consumer was the `linux-amd64-dev` build, relabelled `linux-amd64` so
`logosctl` would load it.

- `resolveRegion china/henan` through the consumer returned the on-chain
  entry (CID `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP`, version
  `261031`, hosted).
- `downloadRegion china/henan <dir>` through the consumer failed: the
  consumer's call to `osm` timed out after 30 s (`RPC_FAILED`), no file was
  written, and the session daemon exited afterwards. Not yet diagnosed.

Evidence for the 1.0.1 recheck:

| File | Shows |
| --- | --- |
| `basecamp-1.0.1-countries-hosted-testnet-v0.3.png` | country rows, e.g. thailand, pakistan, kenya, `hosted · 261031` |
| `basecamp-1.0.1-subregions-hosted-testnet-v0.3.png` | India and China subregions `hosted · 261031` |
| `basecamp-1.0.1-download-china-henan-started.png` | Download clicked, `downloading china/henan…` |
| `basecamp-1.0.1-download-china-henan-error.png` | the `invalid module JSON: unterminated object` error |
| `logosctl-consumer-resolve-china-henan.json` | consumer `resolveRegion` output |
| `logosctl-consumer-download-china-henan.json` | consumer `downloadRegion` error |

## Recheck with osm and osm-app 1.0.2

Rechecked on 2026-11-02 with the 1.0.2 catalog packages, which were built from
this branch (see [`catalog.md`](catalog.md)), against the same program on
testnet v0.3. Basecamp ran on a virtual X display. The storage backend was a
`logosctl` 0.3.2 node on the same machine that hosts the regions
(`LOGOSCTL_BIN`, `LOGOSCTL_CONFIG_DIR`). So the download went through Logos
Storage, but not from a remote peer.

- The registry now has 28 entries: the earlier 25 at `261031`, plus greece,
  chile and belarus at `261101`. After **Refresh**, the app shows them as
  hosted, for example greece `hosted · 261101` and hungary, romania, bulgaria
  and south-korea `hosted · 261031`. Refresh takes about 60 to 90 seconds,
  because it resolves each region with its own `osm_registry_cli` call.
- **Download** on `china/henan` wrote `china-henan-261031.osm.pbf` (50,468,082
  bytes) to the download folder. The app reported `verified: yes` and
  `checksum verified`. Its MD5, `54c5fbf47dc51588de2ffb0a0a94a771`, matches the
  registry entry.
- `osm_consumer_example.downloadRegion china/henan` under `logosctl`, with
  `osm` 1.0.2 and the consumer 1.0.1, returned `{"bytes":50468082,
  "verified":true}` after `setConfig` pointed `logosctl_bin` and
  `storage_config_dir` at the hosting node. The file's MD5 matches. In a second
  run, the session's own fresh `storage_module` was used instead. There,
  `logosctl call` gave up after its fixed 20 s timeout, and after 4 minutes the
  file was still empty. That fresh node had not fetched the content.
- **Host selected** was not clicked. macOS was not checked.

| File | Shows |
| --- | --- |
| `basecamp-osm-app-1.0.2-hosted-greece-hungary-romania-bulgaria.png` | greece `hosted · 261101`; hungary, romania, bulgaria `hosted · 261031` |
| `basecamp-osm-app-1.0.2-hosted-south-korea.png` | south-korea `hosted · 261031` |
| `basecamp-osm-app-1.0.2-hosted-asia-americas-africa.png` | chile `hosted · 261101`, other hosted countries |
| `basecamp-osm-app-1.0.2-subregions-hosted.png` | hosted india and china subregions, including china/henan |
| `basecamp-osm-app-1.0.2-download-verified-china-henan.png` | `downloaded china/henan to ... (verified: yes)`, `checksum verified` |
| `osm_registry_cli-get-testnet-v0.3-28-entries.json` | `osm_registry_cli get` with all 28 entries |
| `consumer-1.0.2-downloadRegion.txt` | both consumer runs, with the MD5 |

## Consumer app from the catalog (SDK embedding example)

Checked on 2026-11-02 with the catalog packages `osm` 1.0.3,
`osm_consumer_example` 1.0.2 and `osm-consumer-app` 1.0.0, against the same
program on testnet v0.3 and the same hosting `logosctl` node as the 1.0.2
recheck. Each `.lgx` was downloaded from its catalog release and its SHA-256
matched `index.json` before install. Basecamp ran on a virtual X display with
the `linux-amd64` variants.

`osm-consumer-app` (see [`sdk.md`](sdk.md#embedding-example-a-basecamp-ui-app-on-top-of-the-consumer))
is a QML app that depends only on `osm_consumer_example`. Its backend calls
the consumer's `configure`, `resolveRegion` and `downloadRegion`, and the
consumer calls the `osm` module through the generated SDK client.

- The `osm-consumer-app` tab loads and reports `ready (program C31bCPXK…)`
  after the consumer forwarded the config to `osm.setConfig`.
- **Resolve** on `china/henan` shows the registry entry: version `261031`,
  checksum `54c5fbf47dc51588de2ffb0a0a94a771`, hosted.
- **Download** wrote `china-henan-261031.osm.pbf` (50,468,082 bytes) and
  reported `downloaded china/henan from logos_storage, 50468082 bytes
  (verified: yes)`. The file's MD5 matches the registry checksum.
- macOS was not run. The `darwin-arm64` variants of these packages are built
  in CI and published, but no one loaded them in Basecamp on a Mac.

| File | Shows |
| --- | --- |
| `sdk-catalog-consumer-app-loaded.png` | osm-consumer-app tab, `ready (program …)` |
| `sdk-catalog-consumer-app-resolve-china-henan.png` | `resolved china/henan` with the registry entry |
| `sdk-catalog-consumer-app-download-verified-china-henan.png` | `downloaded … (verified: yes)`, checksum verified |
| `sdk-catalog-consumer-app-download-china-henan.txt` | installed versions, file size and MD5 |
| `catalog-sha256-check-1.0.3.txt` | downloaded `.lgx` SHA-256 against `index.json` |

## What had to change

Before this check, the app could not run in current Basecamp:

1. `osm-app` was a hand-written UI plugin that relied on an `appBridge`
   context property that Basecamp no longer provides. It now runs its backend
   as a Basecamp core module (`OsmAppBackend`, `"backend": "module"` in
   `metadata.json`), and the QML binds to it with `logos.module("osm-app")`.
2. `OsmFfiClient` called `dlsym(RTLD_DEFAULT, nullptr)` to clear loader
   errors, which segfaults in glibc on the first call. It now calls
   `dlerror()`.
3. The app sends `osm.setConfig` from `OSM_PROGRAM_ID` and `OSM_REGISTRY_CLI`
   so the module knows which program and CLI to use.
4. The vendored `osm-module/lib/libosm_ffi.so` predated the LEZ v0.3.1 port
   and still called the old SPEL CLI (`-i <idl>`). It was rebuilt from `ffi/`.
5. Basecamp refused to open the app until `storage_module` was available
   ("Missing Dependencies: storage_module").

## Reproduce

Pins used: logos-basecamp `66505d5`, logosctl `0.3.2` (storage_module
`3.0.2`), LEZ `v0.3.1`, program `C31bCPXKa5uy1hzmegqMABWLbavoSJbY4Rhrc5yNXPrY`.

```bash
# 1. Basecamp (portable build, so it accepts linux-amd64 modules)
git clone https://github.com/logos-co/logos-basecamp && cd logos-basecamp
git checkout 66505d5
nix build .#dev-portable -o ~/bc/basecamp
cd -

# 2. osm-app and osm modules from this repo
nix build ./osm-app#install-portable    -o ~/bc/osm-app-install
nix build ./osm-module#install-portable -o ~/bc/osm-install

# 3. storage_module 3.0.2 from the pinned logosctl 0.3.2 release
tools/coverage.sh setup     # unpacks logosctl under ~/.cache/logos-osm/tools
mkdir -p ~/bc/deps/modules
cp -r ~/bc/osm-install/modules/osm ~/bc/deps/modules/
cp -r ~/.cache/logos-osm/tools/logosctl-0.3.2/squashfs-root/usr/modules-pkg/storage_module \
      ~/bc/deps/modules/
chmod -R u+w ~/bc/deps

# 4. Registry CLI and a throwaway, unfunded wallet (reads never sign)
(cd examples && RISC0_SKIP_BUILD=1 cargo build --release --bin osm_registry_cli \
   && RISC0_SKIP_BUILD=1 cargo build --release -p wallet --bin wallet)
mkdir -p ~/bc/wallet
cat > ~/bc/wallet/wallet_config.json <<'JSON'
{"sequencer":{"sequencer_addr":"https://testnet.lez.logos.co/"},
 "seq_poll_timeout":"12s","seq_tx_poll_max_blocks":5,
 "seq_poll_max_retries":5,"seq_block_poll_max_amount":100,
 "gas_limit":2000000}
JSON
LEE_WALLET_HOME_DIR=~/bc/wallet examples/target/release/wallet account list
# asks for a password once and creates storage.json

# 5. Cross-check from the command line
export OSM_PROGRAM_ID=C31bCPXKa5uy1hzmegqMABWLbavoSJbY4Rhrc5yNXPrY
export OSM_RPC_URL=https://testnet.lez.logos.co
export OSM_REGISTRY_CLI=$PWD/examples/target/release/osm_registry_cli
export LEE_WALLET_HOME_DIR=~/bc/wallet
"$OSM_REGISTRY_CLI" get > ~/bc/registry.json   # 25 entries, hosted, version 261031

# 6. Launch Basecamp in standalone mode with the app
~/bc/basecamp/bin/logos-basecamp-dev-portable \
  --module ~/bc/osm-app-install \
  --modules-dir ~/bc/deps/modules \
  --user-dir ~/bc/userdata
```

Click **Refresh** in the osm-app tab. Hosted regions show `hosted · <version>`.
The log under `~/bc/userdata/logs/` should contain `Module loaded:
storage_module`, `Module loaded: osm` and `Module loaded: osm-app_backend`.
