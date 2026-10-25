# logos-osm — OpenStreetMap Integration for Logos (LP-0018)

A decentralized OpenStreetMap data distribution system for the Logos ecosystem:
browse available map regions, host Geofabrik PBF snapshots in Logos Storage, and
download map files whose integrity is guaranteed by content-addressing (CID),
with import-time MD5 verification against Geofabrik.

Submission for [LP-0018](docs/LP-0018.md) (OpenStreetMap Integration —
Decentralized Map Data Distribution). Dual licensed under
[MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).

```
fetch (Geofabrik) → verify (MD5) → store (Logos Storage) → register (LEZ program)
                                                     ↓
                          download by CID (or Geofabrik fallback)
```

## Components

| Path | Purpose |
|---|---|
| `osm_registry_core/` | Shared on-chain/off-chain types (`RegionEntry`, `Registry`, error codes) |
| `methods/guest/` | On-chain OSM registry — SPEL/LEZ RISC0 guest program |
| `examples/` | SPEL-generated IDL tooling + instruction CLI (`osm_registry_cli`) |
| `osm_sdk/` | Registry SDK: region discovery, Geofabrik index/checksums/versions, storage backends, host/download workflows |
| `osm_cli/` | CLI: `discover`, `host`, `import`, `lookup`, `check-updates`, `download`, `init-registry` |
| `ffi/` | C ABI bridge over the SDK for Basecamp modules |
| `osm-module/` | Basecamp core module (C++/Qt) — consumer subset + host/import |
| `osm-app/` | OSM distribution Basecamp app (QML) |
| `regions.json` | The frozen, non-overlapping 72-region predefined set |
| `idl/` | Committed SPEL IDL for the registry program |

## The predefined region set

LP-0018 measures coverage against a **closed, non-overlapping set of 72
regions**: 48 countries at country level plus 24 subregions for the four
decomposed countries (8 US states, 6 India zones, 6 China provinces, 4 Russia
federal districts). No region in the set contains another, so no geography is
ever hosted or counted twice. The set is reconstructed verbatim from the prize
document in [`regions.json`](regions.json).

> **Spec deviation note.** Two country-level members of the frozen set have no
> standalone Geofabrik extract: `ireland` (Geofabrik publishes only
> `ireland-and-northern-ireland`) and `malaysia` (only
> `malaysia-singapore-brunei`). Registry keys stay `ireland` and `malaysia`
> exactly as the spec freezes them; checksum and download URLs resolve through
> the mapped combined extract.

## Prerequisites

- Rust (stable) — host crates, CLI, FFI
- Docker + `cargo install --locked cargo-risczero` — guest builds (`make build`)
- [`lgs`](https://github.com/logos-co/logos-scaffold) (`cargo install --git https://github.com/logos-co/logos-scaffold logos-scaffold`) — localnet, wallet, deploy, Basecamp
- Nix with flakes — Basecamp module/app builds
- For the CLI's storage: the headless `logoscore` binary + the built
  `storage_module` (see the [Logos Storage docs](https://logos-storage-docs.netlify.app/tutorials/storage-module/))

## Build, deploy, run

### 1. Build the guest program and generate the IDL

```sh
make build        # RISC0 guest via docker; prints the deterministic Program ID
make idl          # regenerate idl/osm-registry.idl.json from the guest source
make inspect      # Program ID (hex)
```

### 2. Localnet (standalone sequencer)

```sh
lgs setup
lgs localnet start
make deploy                        # deploys the built guest binary
lgs wallet -- account new public
lgs wallet topup 1
```

### 3. Initialize the registry and use the CLI

```sh
cargo build -p osm_cli --bin osm
./target/debug/osm init-registry --program-id <PROGRAM_ID> --signer <ACCOUNT_ID>

./target/debug/osm discover
./target/debug/osm host germany kenya --exclude kenya   # opt-out example
./target/debug/osm import nigeria ./nigeria-latest.osm.pbf
./target/debug/osm lookup --parent india
./target/debug/osm lookup --cid <CID>
./target/debug/osm check-updates
./target/debug/osm download germany --dest ./downloads
```

CLI config lives in `osm.toml` (see `osm_cli/src/main.rs` for the schema) or
environment variables (`OSM_PROGRAM_ID`, `OSM_SIGNER`). `--storage-memory`
runs the whole pipeline without logoscore/lgs (dev mode). `--geofabrik-base`
points the Geofabrik client at a mirror.

Exit codes: `0` ok · `1` usage/error · `2` checksum mismatch ·
`3` storage failure after retries · `4` registry error.

### 4. Testnet 0.4

The registry deploys unchanged to the Logos testnet 0.4 LEZ zone: point `lgs`
at the testnet sequencer (see `docs/configuration.md` in the
[logos-scaffold](https://github.com/logos-co/logos-scaffold) repository for
the current endpoint config), run `make deploy`, and record the resulting
**Program ID** here:

```
Program ID: <set by `make inspect` after the first testnet deploy>
Registry PDA (seed "registry"): <set after init-registry>
```

The program ID is deterministic for a given guest build, so reproducing
`make build` at this commit reproduces the on-chain program exactly.

### 5. Basecamp module + app

```sh
make ffi                                            # builds libosm_ffi.so into osm-module/vendored
nix build .#osm-install .#osm-app-install           # module + QML app
lgs basecamp install osm
lgs basecamp install osm-app
lgs basecamp launch
```

Publishing to a module catalog (fork of `logos-modules-release-base`,
`logos-repo.json` URL for `lgpd`) is documented in
[docs/catalog.md](docs/catalog.md).

## SDK

See [docs/sdk.md](docs/sdk.md) — including the worked example Basecamp
consumers will copy: **resolve a region to its CID and metadata, then
download by CID**. The `osm` module exposes the consumer subset
(discover / resolve / query / download) to other Basecamp apps without the
distribution app.

## On-chain interface

- `init_registry` — permissionless, idempotent; creates the registry PDA.
- `register_region(region, parent, level, cid, source_url, checksum, version, hosted, timestamp, registered_by)` — upsert one entry.
- `register_batch(payload_hex, registered_by)` — upsert up to 72 entries in
  one transaction; payload is `hex(borsh(Vec<RegionEntry>))`.

State: single PDA at seed `[b"registry"]`, borsh `BTreeMap<String, RegionEntry>`
(deterministic serialization), well under the 100 KiB account ceiling at the
full 72-region set. Queries are off-chain: read the account, borsh-decode,
filter by region/parent/CID (`query_by_parent` results are timestamp-ordered).

Cycle counts per instruction are logged from the guest and published in
[docs/cycles.md](docs/cycles.md) (regenerated by CI).

## Region list

48 countries: `germany, france, great-britain, italy, spain, poland,
netherlands, belgium, switzerland, austria, czech-republic, sweden, norway,
denmark, finland, portugal, greece, ireland*, hungary, romania, bulgaria,
ukraine, belarus, turkey, canada, mexico, japan, south-korea, indonesia,
thailand, vietnam, malaysia*, philippines, pakistan, bangladesh, iran,
australia, brazil, argentina, colombia, peru, chile, south-africa, egypt,
nigeria, kenya, morocco, ethiopia` (* resolved through a combined extract —
see the deviation note).

24 subregions: `us/california, us/texas, us/florida, us/new-york,
us/washington, us/illinois, us/georgia, us/pennsylvania`,
`india/{central,eastern,north-eastern,northern,southern,western}-zone`,
`china/{guangdong, jiangsu, shandong, zhejiang, sichuan, henan}`,
`russia/{central, northwestern, volga, siberian}-fed-district`.

## Development

```sh
cargo test --workspace                       # host-side unit + integration tests
cargo test -p osm_e2e --features e2e         # e2e pipeline (in-memory)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI (`.github/workflows/ci.yml`) runs: fmt/clippy/test on ubuntu + windows;
the chain job (docker guest build → IDL staleness check → `lgs localnet` →
deploy → on-chain e2e → cycle-count capture); nix module builds on linux +
macOS; and a dispatch-only live-Geofabrik job so the default-branch CI never
depends on network flakiness.

## Status

- [x] Registry LEZ program (SPEL), IDL committed, staleness-checked in CI
- [x] SDK with consumer + distribution faces, dual backends, retry/back-off
- [x] CLI (7 subcommands, JSON output, exit codes)
- [x] FFI bridge + Basecamp core module + QML distribution app
- [x] E2E pipeline tests (in-memory CI + on-chain CI job)
- [ ] Testnet 0.4 deploy (operator step; program ID recorded above afterwards)
- [ ] Catalog publish (docs/catalog.md) + adoption evidence (15 countries /
      25 verified entries; 5 independent consumer modules)

## License

`MIT OR Apache-2.0` (dual licensed, per LP-0018 eligibility).
