# logos-osm

**OpenStreetMap data distribution for the Logos ecosystem — LP-0018.**

logos-osm implements a decentralized distribution layer for OpenStreetMap
extracts: regions are fetched from Geofabrik, verified against its published
MD5 checksums, stored in Logos Storage, and registered on-chain in a Logos
Execution Zone (LEZ) program. Downloads are content-addressed — a CID from
Logos Storage is the trust anchor — so a hosted region can be re-fetched and
verified without contacting any central operator.

[![ci](https://github.com/YodHeVauHe/logos-osm/actions/workflows/ci.yml/badge.svg)](https://github.com/YodHeVauHe/logos-osm/actions/workflows/ci.yml)
![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)

```
Geofabrik ──fetch──▶ verify (MD5) ──store──▶ Logos Storage ──register──▶ OSM registry (LEZ)
                                                                                        │
user ◀── download by CID (or Geofabrik fallback) ◀── query by region / parent / CID ────┘
```

## Repository layout

| Path | Purpose |
|---|---|
| [`methods/guest`](methods/guest) | On-chain OSM registry — SPEL/LEZ RISC0 guest program |
| [`osm_registry_core`](osm_registry_core) | Shared on-chain/off-chain types (`RegionEntry`, `Registry`, error codes) |
| [`osm_sdk`](osm_sdk) | Registry SDK: discovery, Geofabrik index/checksums/versions, storage backends, host/download workflows |
| [`osm_cli`](osm_cli) | Command-line interface (`osm`) |
| [`ffi`](ffi) | C ABI bridge over the SDK for Basecamp modules |
| [`osm-module`](osm-module) | Basecamp core module (C++/Qt) |
| [`osm-app`](osm-app) | OSM distribution Basecamp app (QML) |
| [`examples`](examples) | SPEL IDL tooling and the generated instruction CLI |
| [`regions.json`](regions.json) | The frozen predefined region set (source of truth) |
| [`idl`](idl) | Committed SPEL IDL for the registry program |

Documentation: [SDK guide](docs/sdk.md) · [Module catalog publishing](docs/catalog.md) ·
[Measured cycle counts](docs/cycles.md) · [FURPS self-assessment](docs/furps.md) ·
[Prize specification](docs/LP-0018.md) · [Implementation plan](docs/plan.md)

## The predefined region set

Coverage is measured against a closed, non-overlapping set of **72 regions**,
reconstructed verbatim from the prize document: 48 countries at country level
and 24 subregions belonging to the four decomposed countries (8 US states,
6 India zones, 6 China provinces, 4 Russia federal districts). No member of
the set contains another, so a given geography is hosted and counted at most
once. The full list is in [`regions.json`](regions.json); the registry keys
are the Geofabrik paths as frozen in the prize document (for example
`germany`, `us/california`, `india/central-zone`).

Two set members have no standalone Geofabrik extract:

| Registry key | Geofabrik extract used |
|---|---|
| `ireland` | `ireland-and-northern-ireland` |
| `malaysia` | `malaysia-singapore-brunei` |

The registry keys remain exactly as frozen in the prize document; only the
download URL and checksum resolution map through the combined extract. This
deviation is disclosed here and in the SDK documentation.

## Getting started

### Prerequisites

- **Rust** (stable) — host crates, CLI, and FFI
- **Docker** with `cargo-risczero` and the `rzup`-installed RISC0 Rust
  toolchain — guest builds (`make build`)
- **[lgs](https://github.com/logos-co/logos-scaffold)** (logos-scaffold) —
  localnet, wallet, deployment, Basecamp installation
- **Nix** with flakes enabled — Basecamp module and app builds
- **logoscore** binary plus the built `storage_module` — headless storage for
  the CLI ([storage docs](https://logos-storage-docs.netlify.app/tutorials/storage-module/))

### Build and deploy

```sh
# 1. Guest program (deterministic; prints the Program ID)
make build
make idl          # regenerate the committed IDL
make inspect      # Program ID (hex)

# 2. Local sequencer
lgs setup
lgs localnet start
make deploy
lgs wallet -- account new public
lgs wallet topup 1

# 3. CLI
cargo build -p osm_cli --bin osm
./target/debug/osm init-registry --program-id <PROGRAM_ID> --signer <ACCOUNT_ID>
```

### CLI usage

```sh
osm discover                       # the 72-region frozen set (URLs, levels)
osm host germany kenya             # fetch → verify → store → one batch tx
osm host germany --exclude kenya   # per-region opt-out
osm import nigeria ./nigeria-latest.osm.pbf
osm lookup --parent india          # timestamp-ordered subregions
osm lookup --cid <CID>
osm check-updates                  # central versions vs registry entries
osm download germany --dest ./downloads
```

Configuration is read from `osm.toml` or environment variables
(`OSM_PROGRAM_ID`, `OSM_SIGNER`). `--storage-memory` runs the full pipeline
without logoscore or lgs (development mode); `--geofabrik-base` directs the
Geofabrik client at a mirror. Exit codes: `0` success, `1` usage or error,
`2` checksum mismatch, `3` storage failure after retries, `4` registry error.

### Basecamp module and app

```sh
make ffi                                    # builds libosm_ffi.so into osm-module/vendored
nix build .#osm-install .#osm-app-install   # module + QML app
lgs basecamp install osm
lgs basecamp install osm-app
lgs basecamp launch
```

Publishing to a module catalog (a fork of
[`logos-modules-release-base`](https://github.com/logos-co/logos-modules-release-base)
via
[`logos-modules-release-action`](https://github.com/logos-co/logos-modules-release-action),
installable with `lgpd`) is documented in
[docs/catalog.md](docs/catalog.md).

### Testnet 0.4

The registry deploys to the Logos testnet 0.4 LEZ zone unchanged: configure
`lgs` for the testnet sequencer (see the
[logos-scaffold configuration docs](https://github.com/logos-co/logos-scaffold)),
run `make deploy`, then record the resulting identifiers:

```
Program ID:                     <set by `make inspect` after the first testnet deploy>
Registry PDA (seed "registry"): <set after init-registry>
```

The Program ID is deterministic for a given guest build, so reproducing
`make build` at this commit reproduces the on-chain program exactly.

## On-chain interface

State lives in a single PDA at seed `[b"registry"]`: a borsh-serialized
`BTreeMap<String, RegionEntry>` (deterministic across executions), well below
the 100 KiB account ceiling at the full 72-region set.

| Instruction | Description |
|---|---|
| `init_registry` | Permissionless, idempotent; creates the registry PDA |
| `register_region` | Upsert one region entry (all spec fields: `region`, `parent`, `level`, `cid`, `source_url`, `checksum`, `version`, `hosted`, `timestamp`) |
| `register_batch` | Upsert up to 72 entries in one transaction; payload is `hex(borsh(Vec<RegionEntry>))` |

Queries are off-chain: read the account, borsh-decode, filter by region,
parent, or CID (`query_by_parent` results are timestamp-ordered). Cycle
counts for every instruction are logged from the guest and published in
[docs/cycles.md](docs/cycles.md), regenerated by CI on each run.

## Development

```sh
cargo test --workspace                            # unit + integration tests
cargo test -p osm_e2e --features e2e              # e2e pipeline (in-memory)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI (`.github/workflows/ci.yml`) runs on every push:

- **rust** — fmt, clippy, tests, and the e2e suite on Ubuntu and Windows
- **chain** — Docker guest build, IDL staleness check, `lgs localnet`
  deployment, on-chain end-to-end test, cycle-count capture
- **nix** — Basecamp module and app builds on Linux and macOS
- **live-geofabrik** — dispatch-only smoke test against the real Geofabrik
  service, so the default-branch CI never depends on network flakiness

## Status

- [x] Registry LEZ program (SPEL) with committed, staleness-checked IDL
- [x] SDK with consumer and distribution faces; dual storage/registry backends
- [x] CLI (seven subcommands, JSON output, documented exit codes)
- [x] FFI bridge, Basecamp core module, QML distribution app
- [x] End-to-end pipeline tests (in-memory CI; on-chain CI job)
- [ ] Testnet 0.4 deployment (operator step; identifiers recorded above)
- [ ] Module catalog publication and adoption evidence (15 countries, 25
      verified entries, 5 independent consumer modules)

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), per
LP-0018 eligibility requirements.
