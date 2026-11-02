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
- Not covered here: the **Host selected** button, which needs a funded
  wallet; macOS. **Download** is covered by the 1.0.2 recheck below.

Evidence (in [`evidence/basecamp/`](evidence/basecamp/)):

| File | Shows |
| --- | --- |
| `basecamp-portable-osm-app-loaded-no-bridge.png` | osm-app tab loaded through the module backend, before the region list |
| `basecamp-portable-osm-app-discover.png` | predefined regions listed from `osm.discover` |
| `basecamp-osm-app-registry-lookup-testnet-v0.3.png` | registry lookup against testnet v0.3, hosted regions marked |
| `basecamp-osm-app-kenya-hosted-testnet-v0.3.png` | e.g. kenya, egypt, pakistan shown `hosted · 261031` |
| `osm_registry_cli-get-testnet-v0.3.json` | `osm_registry_cli get` output for the same program |

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
