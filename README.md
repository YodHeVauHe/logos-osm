# Solution: LP-0018 — OpenStreetMap Integration: Decentralized Map Data Distribution

**Submitted by:** [YodHeVauHe](https://github.com/YodHeVauHe) · **Repository:** <https://github.com/YodHeVauHe/logos-osm> · **Prize:** [LP-0018 — OpenStreetMap Integration](docs/LP-0018.md) · **License:** [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)

[![ci](https://github.com/YodHeVauHe/logos-osm/actions/workflows/ci.yml/badge.svg)](https://github.com/YodHeVauHe/logos-osm/actions/workflows/ci.yml)
![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)

`logos-osm` is a Logos-native distribution layer for OpenStreetMap extracts. It
turns Geofabrik's published PBF snapshots into content-addressed, on-chain
discoverable regions: fetch → verify against Geofabrik's MD5 → store in Logos
Storage → register in a Logos Execution Zone (LEZ) program. Consumers resolve a
region to its CID and metadata, then download by content address without
contacting a central operator.

The deliverable spans the full LP-0018 surface: an on-chain registry written
with SPEL, a Rust SDK/module for Basecamp consumers, a CLI and a C FFI bridge,
plus a Basecamp core module and a QML distribution app.

## Summary

The implementation targets the prize's closed, non-overlapping set of **72
regions** — 48 country-level extracts and 24 subregions of the four decomposed
countries (US, India, China, Russia). Coverage is measured against that frozen
set, so no geography is hosted or counted twice.

What ships in this repository:

- **Registry program** (`methods/guest`) — an LEZ/RISC0 guest written with
  SPEL, holding all region entries in one deterministic PDA.
- **Shared types** (`osm_registry_core`) — `RegionEntry`, `Registry`,
  validation rules and stable error codes, compiled for both guest and host.
- **SDK** (`osm_sdk`) — region discovery, Geofabrik index/checksum/version
  resolution, pluggable storage and registry backends, and the host, bulk-host,
  import, update-check and download workflows.
- **CLI** (`osm_cli`) — seven subcommands with JSON output and documented exit
  codes.
- **FFI + Basecamp artifacts** (`ffi`, `osm-module`, `osm-app`) — a C ABI over
  the SDK, a core module exposing the consumer subset plus the host/import
  calls used by the app, and a QML distribution app.
- **Verification** — 49 tests across the unit, integration and e2e suites; the
  in-memory e2e pipeline runs in CI, and the chain job runs it against a local
  LEZ sequencer.

The code paths required by LP-0018 are implemented and tested. Deployment to
Logos testnet 0.4, publication to a module catalog, measured cycle counts and
the adoption criteria are operator steps that are still open; they are listed
explicitly in [Status and roadmap](#status-and-roadmap) rather than claimed as
complete.

## Approach

### System shape

```
Geofabrik ──fetch──▶ verify (MD5) ──store──▶ Logos Storage ──register──▶ OSM registry (LEZ)
                                                                                     │
consumer ◀── download by CID (Geofabrik fallback) ◀── query by region / parent / CID ─┘
```

The same pipeline is exposed three ways: the CLI calls the SDK directly, the
Basecamp app calls the C++ module, and the module calls the Rust SDK through a
small C ABI. No business logic is duplicated across those surfaces.

### Registry: one deterministic PDA

All state lives in a single PDA at seed `[b"registry"]` as a borsh-serialized
`Registry { entries: BTreeMap<String, RegionEntry> }`. `BTreeMap` gives
deterministic bytes regardless of insertion order, which matters for a guest
program whose execution must be reproducible. The guest guards every write
against the LEZ 100 KiB account ceiling; the full 72-region set is roughly
19 KB, so there is ample headroom.

A per-region PDA design was rejected: it would make enumeration a client-side
indexing problem, require one transaction per region for bulk hosting, and lose
the atomicity that LP-0018 asks for. A single account keeps `register_batch`,
discovery and account reads simple at the cost of a coarser write unit, which is
acceptable for a set this small.

### Writes on-chain, reads off-chain

Only mutations are transactions. Discovery, lookup by region, lookup by parent
and lookup by CID are borsh reads of the registry account followed by filtering
in the client. This avoids paying execution cost for read-only queries and
keeps the on-chain interface minimal: `init_registry`, `register_region` and
`register_batch`.

`register_batch` takes `hex(borsh(Vec<RegionEntry>))` rather than a text
encoding. Region paths contain `/`, URLs contain `:` and `/`, and checksums are
hex — delimiter-based payloads would be fragile. The guest caps a batch at 72
entries (`MAX_BATCH`) and validates every entry before writing.

### Trust model: CID first, MD5 at import time

A CID from Logos Storage is the trust anchor. Once a region is hosted, a
consumer can download it and verify the content address without contacting
Geofabrik. Geofabrik's published MD5 is evidence of provenance and is checked
at import/host time; it is deliberately not a runtime dependency for downloads.

This split matters for the prize's "no mandatory centralised external
services" requirement: Geofabrik is the canonical snapshot source (index,
checksums and fallback for unhosted regions), but hosted regions remain
discoverable and downloadable when Geofabrik is unavailable.

### The frozen 72-region set

`regions.json` is the single source of truth, embedded into the SDK at compile
time. It contains 48 country entries and 24 subregion entries, and the set is
non-overlapping by construction. URL resolution follows Geofabrik's parent
chain rather than the registry key alone (`india/central-zone` resolves under
`asia/india/`, US states under `north-america/us/`, Russian districts under
`russia/`).

Two frozen keys have no standalone Geofabrik extract and are mapped explicitly:

| Registry key | Geofabrik extract used |
|---|---|
| `ireland` | `ireland-and-northern-ireland` |
| `malaysia` | `malaysia-singapore-brunei` |

The registry keys stay exactly as written in the prize document; only URL and
checksum resolution use the combined extract. The deviation is disclosed here,
in [`docs/sdk.md`](docs/sdk.md) and in the SDK code.

### One SDK, two faces

LP-0018 distinguishes the distribution app from the SDK that other Basecamp
modules consume:

- The **distribution face** covers host, bulk host, local import, update checks
  and download. The CLI, FFI bridge and distribution app use it.
- The **consumer face** covers discovery, resolve (region → CID/metadata),
  query by parent/CID and download. The `osm` module exposes this subset
  alongside the host/import calls the distribution app uses, so another app can
  depend on map data without pulling in the distribution UI.

Storage and registry access sit behind traits (`StorageBackend`,
`RegistryClient`). Production uses `logoscore` with `storage_module` and the
SPEL-generated instruction CLI plus `lgs wallet` reads; tests use an in-memory
storage double and a mock registry. That boundary is what makes the e2e suite
run without a sequencer while the chain CI job exercises the real path.

### Failure handling

Downloads and storage uploads retry transient failures with exponential
back-off (1s, 2s, 4s … capped at 30s) and surface `SdkError::RetriesExhausted`
with the underlying context after the last attempt. Checksum mismatches fail
fast, delete the partial file and never register anything. Bulk hosting skips a
region that fails verification while still registering the rest in one batch
transaction. Registry writes are validated locally with the same
`validate_entry` function the guest uses, so malformed entries fail before a
transaction is submitted.

### Why the Logos stack

The prize is about verifiable data distribution without a trusted operator.
LEZ provides an on-chain registry whose state can be read and audited by any
consumer; Logos Storage provides content-addressed availability, so integrity
survives independent mirroring; Basecamp modules provide the composition layer
through which other apps consume the data. A centralised alternative could
serve files faster, but it would reintroduce the single operator, silent
mutation and takedown risk that this design exists to remove.

### Constraints and things that did not work

- The guest is a `#![no_main]` RISC0 program and must be built with the
  deterministic Docker toolchain; host-side Rust cannot substitute for it. CI
  derives the Program ID from the built binary rather than hard-coding one.
- `spel-framework` master emitted an unresolved `nssa_core` reference, so the
  known-good framework is vendored under `vendor/spel-framework` and pinned.
  This is deliberate: a moving dependency would make the Program ID
  irreproducible.
- The Basecamp module uses the universal authoring model: a Qt-free
  implementation plus a separate Qt interface, with the FFI shared library
  vendored as an external library. The module and QML app are built through
  `logos-module-builder` (`mkLogosModule` / `mkLogosQmlModule`), which requires
  Nix rather than a plain Cargo build.
- IDL generation requires the SPEL toolchain (`make idl`). CI uploads the
  generated file as an artifact and warns on staleness until the generated IDL
  is committed; see the roadmap.

## Success Criteria Checklist

### Functionality

- [x] **Region discovery** — `regions::predefined()` returns the frozen set
  with level, parent, Geofabrik index id and canonical URL; `osm discover
  [--refresh]` optionally refreshes URLs from the live Geofabrik index.
- [x] **Host workflow** — `OsmSdk::host_region` downloads the PBF, verifies the
  published MD5, stores it in Logos Storage and registers the entry on-chain.
- [x] **Download workflow** — `OsmSdk::download_region` reads the CID from
  Logos Storage when the region is hosted, otherwise falls back to Geofabrik;
  the report carries the verification result.
- [x] **Bulk host** — `OsmSdk::host_regions` downloads/verifies/stores all
  selected regions and registers them in **one** `register_batch` transaction;
  the CLI exposes per-region opt-out with `--exclude`, and the SDK keeps a
  structural parent/child overlap guard.
- [x] **Local import** — `OsmSdk::import_local` verifies a caller-supplied PBF
  against Geofabrik's MD5 (or an explicit `--md5` for offline use), then stores
  and registers it by CID.
- [x] **Update check** — `OsmSdk::check_updates` compares each registry entry's
  version against Geofabrik's current dated filename, matched on region path so
  a country and its subregions are tracked independently.
- [x] **On-chain registry** — the SPEL program stores all required fields
  (`region`, `parent`, `level`, `cid`, `source_url`, `checksum`, `version`,
  `hosted`, `timestamp`, plus `registered_by` and `record_version`), keys on the
  Geofabrik path, supports batch registration, and is queryable off-chain by
  region, parent and CID.
- [ ] **Testnet 0.4 end-to-end** — the chain CI job deploys and exercises the
  program against a local LEZ sequencer; the public testnet 0.4 deployment is
  still an operator step (no Program ID is recorded yet).
- [x] **OSM registry SDK** — `osm_sdk` provides the full distribution API; the
  `osm` module exposes the consumer subset; a worked resolve-to-CID example is
  in [`docs/sdk.md`](docs/sdk.md).

### Usability

- [x] **Basecamp app** — `osm-app` is a QML distribution app with region
  discovery, hosted/version status, bulk host with opt-out, local import,
  download result and an update banner; build/install steps are below.
- [x] **SDK/module documentation and embedding example** — API overview and
  worked example in [`docs/sdk.md`](docs/sdk.md); the module is designed to run
  without the distribution app.
- [x] **CLI** — `discover`, `host`, `import`, `lookup`, `check-updates`,
  `download` and `init-registry`, with `--json` on query/report commands.
- [x] **Clear UX** — the CLI prints hosted status, versions and verification
  results; the app shows the same state in the region table and verification
  line.

### Reliability

- [x] **Retries with back-off** — shared `backoff_delays` for Geofabrik
  downloads and Logos Storage uploads; a clear error after exhaustion.
- [x] **Graceful checksum failure** — `ChecksumMismatch { region, expected,
  actual }`, exit code `2` in the CLI, partial file removed, no registration.
- [x] **Timestamp ordering** — registry entries carry `timestamp`; parent
  queries return ascending by timestamp for version sorting.
- [x] **No mandatory centralised services** — only the configured sequencer/LEZ
  node, Logos Storage and Geofabrik are used; no analytics or third-party
  service. Geofabrik can be redirected to a mirror with `--geofabrik-base`.

### Performance

- [ ] **Documented cycle counts** — the guest logs cycle counts for
  `register_region` and `register_batch`, the chain CI job captures them, and
  [`docs/cycles.md`](docs/cycles.md) documents the method. The measured values
  are still CI-generated placeholders and have not been committed.

### Supportability

- [ ] **Deployed and tested against Logos testnet 0.4** — deployment steps and
  the identifiers to record are documented below; the public deployment has not
  been performed from this repository yet.
- [ ] **Platform support (recommended)** — Rust host code is tested on Ubuntu
  and Windows; the module/app are built on Linux. The flake declares Linux and
  macOS outputs, but macOS module builds and Android are not yet validated in
  CI.
- [x] **Built with `logos-module-builder`** — `flake.nix` uses
  `mkLogosModule` for the core module and `mkLogosQmlModule` for the app.
- [ ] **Published to a module catalog** — the fork/release procedure and the
  required `logos-repo.json` URL are documented in
  [`docs/catalog.md`](docs/catalog.md); the catalog has not been published yet,
  so no install URL is recorded here.
- [ ] **IDL via SPEL** — `make idl` generates the IDL from the guest source and
  CI uploads it as an artifact, but the generated file is not yet committed, so
  the staleness check currently warns instead of failing.
- [x] **End-to-end tests in CI** — `tests/e2e` covers host → query → download;
  the `chain` job runs it against a real local sequencer, and the default
  branch CI does not depend on live Geofabrik.
- [ ] **README** — this document covers setup, CLI and app usage, the region
  set and the deployment steps, but the deployed Program ID is pending the
  testnet step above.
- [x] **SDK worked example** — resolve a region to its CID and metadata in
  [`docs/sdk.md`](docs/sdk.md).

### Adoption status

Adoption is measured by evaluators from on-chain data and public repositories;
it is not claimed here until the evidence exists.

- [ ] At least 15 predefined countries covered on Logos testnet 0.4 and hosted
  in Logos Storage, with at least 25 verified region entries.
- [ ] At least 5 independent modules (including 3 Basecamp UI apps) consuming
  the SDK, one of which may use only the standalone SDK module.
- [ ] macOS users or installs represented in the adoption evidence.

## FURPS Self-Assessment

### Functionality

The registry, SDK, CLI, FFI bridge, core module and QML app implement the full
fetch → verify → store → register → download pipeline for the frozen set. The
supported operations are host, bulk host, local import, update check, lookup by
region/parent/CID, download and on-chain initialization. The SDK exposes both
the distribution face and the smaller consumer face described in LP-0018.

Known limits: snapshots only (no live OSM sync), no map rendering/routing/
geocoding, no custom regions or extra tree levels, and no hosted relay. Those
are explicitly out of scope in the prize.

### Usability

A new user can run `cargo build -p osm_cli --bin osm`, then `osm discover` with
no network or chain access to inspect the 72-region set. Hosting and downloads
need a configured LEZ node, signer and Logos Storage; the README lists those
prerequisites and the `--storage-memory` development mode lets the pipeline run
without either. The Basecamp app exposes the same operations through a region
list, action panel and update banner.

### Reliability

Writes are validated twice (SDK and guest) with shared rules, batch payloads are
borsh-encoded rather than delimiter-packed, and serialization is deterministic
through `BTreeMap`. Transient download/upload failures retry with capped
exponential back-off; checksum mismatches fail closed. Hosted downloads do not
need Geofabrik, and the only external endpoints are the user's Logos node,
Logos Storage and (optionally) Geofabrik or a mirror.

### Performance

The full 72-entry registry is far below the 100 KiB account limit, and a batch
registration is one transaction by design. The guest logs cycle counts for
`init_registry`, `register_region` and `register_batch`; the CI chain job
captures them. Measured numbers remain pending and are not presented as
results.

### Supportability

The workspace is split into small crates with one responsibility each: shared
types, SDK, CLI, FFI, guest program, module and app. Tests cover validation,
Geofabrik parsing, storage, registry payloads, workflows and the e2e pipeline;
CI adds fmt, clippy, the Windows Rust matrix, a deterministic guest build, a
local-sequencer e2e run and Nix module/app builds. Docs cover the SDK, catalog
publishing, cycle measurement and the FURPS mapping. The remaining operational
work is deployment, catalog publication and adoption evidence, not missing
implementation.

The full criterion-by-criterion mapping is in [`docs/furps.md`](docs/furps.md).

## Repository layout

| Path | Purpose |
|---|---|
| [`methods/guest`](methods/guest) | On-chain OSM registry — SPEL/LEZ RISC0 guest program |
| [`osm_registry_core`](osm_registry_core) | Shared on-chain/off-chain types, validation and error codes |
| [`osm_sdk`](osm_sdk) | Registry SDK: discovery, Geofabrik, storage backends, registry clients, workflows |
| [`osm_cli`](osm_cli) | Command-line interface (`osm`) |
| [`ffi`](ffi) | C ABI bridge over the SDK for Basecamp modules |
| [`osm-module`](osm-module) | Basecamp core module (C++/Qt) exposing the consumer subset |
| [`osm-app`](osm-app) | OSM distribution Basecamp app (QML) |
| [`examples`](examples) | SPEL IDL generation and the generated instruction CLI |
| [`tests/e2e`](tests/e2e) | Host → query → download end-to-end test |
| [`regions.json`](regions.json) | Frozen predefined region set (source of truth) |
| `idl/osm-registry.idl.json` | Generated SPEL IDL (produced by `make idl`; not yet committed) |
| [`docs`](docs) | SDK, catalog, cycles, FURPS and prize reference |
| [`flake.nix`](flake.nix) | Basecamp module/app builds via `logos-module-builder` |

## Getting started

### Prerequisites

- **Rust stable** — host crates, CLI and FFI.
- **Docker** with `cargo-risczero` and the `rzup`-installed RISC0 Rust
  toolchain — deterministic guest builds (`make build`).
- **SPEL CLI (`spel`)** — Program ID inspection and the deploy helper
  (`make inspect`, `make deploy`); `make idl` builds the generator from the
  vendored framework, so the CLI is only needed for inspection/deployment.
- **[lgs](https://github.com/logos-co/logos-scaffold)** — localnet, wallet,
  deployment and Basecamp installation.
- **Nix with flakes enabled** — Basecamp module and app builds.
- **`logoscore` plus the built `storage_module`** — headless storage for the
  CLI ([storage docs](https://logos-storage-docs.netlify.app/tutorials/storage-module/)).
  Not needed with `--storage-memory`.

### Build and deploy (localnet)

```sh
# 1. Guest program (deterministic; prints the Program ID)
make build
make idl            # regenerate the IDL (requires the SPEL toolchain)
make inspect        # Program ID (hex)

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

The SDK also needs the generated instruction CLI (`make cli`) and, for real
storage, `logoscore` with `storage_module` on its module path. Defaults are
`./target/debug/osm_registry_cli`, `./idl/osm-registry.idl.json`, `./modules`
and `storage.json`; override them in `osm.toml` or with the flags below.

### CLI usage

```sh
osm discover                        # the 72-region frozen set (URLs, levels)
osm discover --refresh              # refresh canonical URLs from Geofabrik
osm host germany kenya              # fetch → verify → store → one batch tx
osm host germany --exclude kenya    # per-region opt-out
osm import nigeria ./nigeria-latest.osm.pbf
osm import kenya ./kenya.osm.pbf --md5 <32-hex>   # offline verification
osm lookup --parent india           # timestamp-ordered subregions
osm lookup --region us/california
osm lookup --cid <CID>
osm check-updates                   # central versions vs registry entries
osm download germany --dest ./downloads
```

Global flags: `--config <path>`, `--program-id <hex>`, `--signer <account>`,
`--storage-memory`, `--geofabrik-base <url>`. Configuration is read from
`./osm.toml` or `~/.config/osm/osm.toml`; environment overrides are
`OSM_PROGRAM_ID`, `OSM_SIGNER` and `OSM_GEOFABRIK_BASE`.

`--storage-memory` uses the in-memory storage double **and** a mock registry, so
it exercises the pipeline without `logoscore` or a chain. It is a development
and test mode, not a way to publish data.

Exit codes: `0` success, `1` usage/other error, `2` checksum mismatch, `3`
storage failure after retries, `4` registry/validation error.

### Basecamp module and app

```sh
make ffi                                      # builds osm-module/lib/libosm_ffi.so
nix build .#osm-install .#osm-app-install     # module + QML app
lgs basecamp install osm
lgs basecamp install osm-app
lgs basecamp launch
```

The core module declares `storage_module` as a dependency; installing
`osm-app` pulls in the module chain. Catalog publication (a fork of
[`logos-modules-release-base`](https://github.com/logos-co/logos-modules-release-base)
using
[`logos-modules-release-action`](https://github.com/logos-co/logos-modules-release-action),
installable with `lgpd`) is documented in [`docs/catalog.md`](docs/catalog.md).

### Testnet 0.4

The guest targets the LEZ zone used by Logos testnet 0.4. Configure `lgs` for
the testnet sequencer (see the
[logos-scaffold configuration docs](https://github.com/logos-co/logos-scaffold)),
run `make deploy`, then record the identifiers:

```
Program ID:                     <pending — set by `make inspect` after the testnet deploy>
Registry PDA (seed "registry"): <pending — set after `init-registry`>
```

The Program ID is deterministic for a given guest build, so rebuilding this
commit with the pinned toolchain reproduces the same Program ID. This
repository does not yet contain a testnet 0.4 deployment or a published
Program ID; the local-sequencer path is what CI exercises today.

## On-chain interface

State lives in a single PDA at seed `[b"registry"]`: a borsh-serialized
`Registry { entries: BTreeMap<String, RegionEntry> }`. The full set is well
below the 100 KiB account ceiling, and the guest refuses writes that would
exceed it.

| Instruction | Description |
|---|---|
| `init_registry` | Permissionless, idempotent; creates the registry PDA |
| `register_region` | Upsert one entry with all required fields |
| `register_batch` | Upsert 1–72 entries in one transaction; payload is `hex(borsh(Vec<RegionEntry>))` |

Queries are off-chain: read the account, borsh-decode, then filter by region,
parent or CID (`query_by_parent` results are timestamp-ordered). Cycle counts
for every instruction are logged from the guest and published in
[`docs/cycles.md`](docs/cycles.md), regenerated by CI.

## Development

```sh
cargo test --workspace                            # unit + integration tests
cargo test -p osm_e2e --features e2e              # e2e pipeline (in-memory)
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

CI (`.github/workflows/ci.yml`) runs on every push:

- **rust** — fmt, clippy, workspace tests and the in-memory e2e suite on Ubuntu
  and Windows.
- **chain** — RISC0 guest build, IDL generation/freshness check, `lgs localnet`,
  deployment, on-chain e2e and cycle-count capture.
- **nix** — Basecamp core module and QML app builds.
- **live-geofabrik** — dispatch-only live checksum smoke test, so default-branch
  CI never depends on network flakiness.

## Status and roadmap

Implemented in-repo:

- [x] SPEL/LEZ registry program with shared validation and stable error codes
- [x] SDK with consumer/distribution faces and pluggable storage/registry
      backends
- [x] CLI with seven subcommands, JSON output and documented exit codes
- [x] FFI bridge, Basecamp core module and QML distribution app
- [x] Unit/integration tests and the in-memory e2e pipeline
- [x] CI workflow covering Rust, the chain/localnet path and Nix module/app
      builds
- [x] SDK, catalog, cycles and FURPS documentation

Open operator steps:

- [ ] Deploy to Logos testnet 0.4 and record the Program ID and registry PDA
- [ ] Generate and commit `idl/osm-registry.idl.json`; make the CI staleness
      check fail closed
- [ ] Capture and commit measured cycle counts in `docs/cycles.md`
- [ ] Publish the modules to a catalog fork and record the `logos-repo.json` URL
- [ ] Produce the adoption evidence: 15 countries / 25 verified entries, five
      independent modules (three Basecamp UI apps) and macOS coverage

## Supporting materials

- [Prize specification](docs/LP-0018.md)
- [SDK guide and worked example](docs/sdk.md)
- [FURPS self-assessment](docs/furps.md)
- [Cycle-count methodology](docs/cycles.md)
- [Module catalog publishing](docs/catalog.md)
- [Implementation plan](docs/plan.md)
- [End-to-end test](tests/e2e/src/lib.rs)
- [CI workflow](.github/workflows/ci.yml)
- [Module metadata](osm-module/metadata.json) and
  [app metadata](osm-app/metadata.json)

## Terms & Conditions

By submitting this solution, I confirm that I have read and agree to the
[Terms & Conditions](https://github.com/logos-co/lambda-prize/blob/master/TERMS.md).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), per
LP-0018 eligibility requirements.
