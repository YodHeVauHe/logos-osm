# Solution: LP-0018 — OpenStreetMap Integration

**Submitted by:** [YodHeVauHe](https://github.com/YodHeVauHe) · **Prize:** [LP-0018](docs/LP-0018.md) · **License:** [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)

[![CI workflow badge](https://github.com/YodHeVauHe/logos-osm/actions/workflows/ci.yml/badge.svg)](https://github.com/YodHeVauHe/logos-osm/actions/workflows/ci.yml) · **Branch evidence:** local Rust and IDL checks pass; GitHub Actions has not run for this branch. Testnet deployment, catalog publication, and adoption evidence are pending owner actions.

`logos-osm` provides a registry and tools for distributing Geofabrik OpenStreetMap PBF snapshots through Logos. It includes a SPEL/LEZ registry, Rust SDK, CLI, C FFI, Basecamp core module, and QML app. The repository implements the workflows; public deployment, catalog release, and ecosystem adoption are not yet evidenced.

## Contents

- [Architecture](#architecture)
- [Components](#components)
- [Prerequisites](#prerequisites)
- [Build and test](#build-and-test)
- [Usage](#usage)
- [Program IDs](#program-ids)
- [Regions](#regions)
- [Success criteria](#success-criteria-checklist)
- [FURPS assessment](#furps-self-assessment)
- [Limitations and owner actions](#limitations-and-owner-actions)
- [Supporting materials](#supporting-materials)
- [Terms and license](#terms-and-license)

## Architecture

```text
Geofabrik ──fetch──▶ MD5 verify ──▶ Storage ──▶ LEZ registry
                                                │
CLI / Basecamp app ◀── SDK / FFI ◀── query CID and metadata
        │                                       │
        └──────────── download by CID ◀────────┘
```

Geofabrik supplies the snapshot index, published MD5 values, and fallback downloads for unhosted regions. Hosted content is addressed by its Storage CID. The registry stores region metadata; clients query registry state off-chain.

## Components

| Component | Role |
|---|---|
| SPEL LEZ program (`methods/guest`) | Initializes the registry and registers one or multiple region entries. |
| `osm_registry_core` | Shared registry types, validation, and error codes. |
| `osm_sdk` | Discovery, verification, storage and registry abstractions, hosting, import, queries, downloads, and update checks. |
| `osm_cli` | `osm` command line interface. |
| `ffi` | C ABI used by the Basecamp core module. |
| `osm-module` | Basecamp module exposing SDK operations. |
| `osm-app` | QML distribution app using the module. |
| `examples/basecamp-consumer` | Independent Basecamp consumer example for the `osm` module. |

## Prerequisites

- Rust stable and Cargo for the workspace, SDK, CLI, FFI, and IDL generator.
- Nix with flakes enabled for Basecamp module and app packages.
- `cargo-risczero`, the RISC Zero toolchain, Docker, SPEL, and `lgs` for guest build and localnet deployment (`make build`, `make deploy`).
- `logoscore` and a built `storage_module` for production Storage operations. The in-memory mode is for development and tests.

## Build and test

Commands verified from this branch:

```sh
cargo build --workspace
cargo test --workspace
make idl
nix flake show
cargo run -p osm_cli --bin osm -- --help
```

`make idl` regenerates the tracked `idl/osm-registry.idl.json`. `nix flake show` lists Basecamp package outputs, including `osm-install`, `osm-app-install`, and `osm-consumer-example`. Nix evaluation reports a deprecation warning for the QML app backend. The three Basecamp packages below were built successfully on x86_64 Linux for this branch:

```sh
nix build .#osm-install .#osm-app-install
nix build .#osm-consumer-example
```

Run the in-memory end-to-end suite with `cargo test -p osm_e2e --features e2e`. The real localnet registry test and the full real Storage roundtrip require configured Logos services; the latter is not yet verified because the `LogoscoreStorage` adapter targets the legacy `uploadFile`/`downloadFile`/`nameObject` API and the current Storage module exposes `uploadUrl`/`downloadToUrl`.

## Usage

### CLI

Build the CLI with `cargo build -p osm_cli --bin osm`; run it as `./target/debug/osm`.

```sh
./target/debug/osm discover
./target/debug/osm discover --refresh
./target/debug/osm lookup --parent india --program-id <PROGRAM_ID>
./target/debug/osm download germany --dest ./downloads --program-id <PROGRAM_ID>
./target/debug/osm host germany --program-id <PROGRAM_ID> --signer <ACCOUNT_ID>
```

`discover` lists the frozen set without a registry connection. Registry and hosting commands need a configured program ID and signer; hosting also needs a working Logos Storage setup. See [`docs/sdk.md`](docs/sdk.md) for SDK configuration and API details. `--storage-memory` selects test doubles and is not a publishing mode.

### Basecamp app

Build the FFI library with `make ffi`, then build the app package with `nix build .#osm-app-install`. Install and launch the resulting app through Basecamp; the app provides region discovery, hosting, import, download, and update-check controls. The package build and interactive runtime have not been verified on this branch.

## Program IDs

**Logos testnet 0.4:** pending owner deployment. No testnet Program ID or registry PDA is recorded.

**Localnet measurement only:** `8fd44f40975a635ba9c4a2fb8f91118c0cfcc060831e1a964d8d1583abadbad4`. This is the guest Program ID associated with the measured localnet samples in [`docs/cycles.md`](docs/cycles.md); it is not evidence of testnet deployment.

## Regions

The closed, non-overlapping set contains 72 entries: 48 country-level extracts and 24 subregions for the United States, India, China, and Russia. [`regions.json`](regions.json) is the source of truth; the prize specification lists the regions by name in [Predefined Regions](docs/LP-0018.md#predefined-regions-closed-set). No custom regions are included.

## Success Criteria Checklist

Status is based on repository artifacts and commands run locally. A checked implementation item does not imply a successful GitHub Actions run or public deployment.

| Requirement | Status | Evidence |
|---|---|---|
| SPEL registry with required fields and batch registration | Implemented; Rust workspace tests pass | [`methods/guest`](methods/guest), [`osm_registry_core`](osm_registry_core); `cargo test --workspace` |
| Frozen region discovery, checksums, hosting/import, queries and updates | Implemented; Rust workspace tests pass | [`regions.json`](regions.json), [`osm_sdk`](osm_sdk), [`osm_cli`](osm_cli) |
| CLI and consumer SDK | Implemented; CLI help and workspace build verified | [`osm_cli`](osm_cli), [`docs/sdk.md`](docs/sdk.md) |
| Basecamp core module, app, and independent consumer example | Source and flake outputs present; Nix build/runtime not verified here | [`osm-module`](osm-module), [`osm-app`](osm-app), [`examples/basecamp-consumer`](examples/basecamp-consumer), [`flake.nix`](flake.nix) |
| SPEL IDL generated and tracked | `make idl` passes locally; CI result pending | [`idl/osm-registry.idl.json`](idl/osm-registry.idl.json) |
| In-memory e2e | Test is available; workspace tests pass. Dedicated feature-gated e2e command not run in this docs pass | [`tests/e2e`](tests/e2e) |
| Real sequencer e2e and cycle capture | Configured in workflow; GitHub Actions has not run on this branch. Localnet measurements are documented | [CI workflow](.github/workflows/ci.yml), [`docs/cycles.md`](docs/cycles.md) |
| Logos Storage host-query-download roundtrip | Pending adapter compatibility work and verification | [`osm_sdk/src/storage.rs`](osm_sdk/src/storage.rs), current API note above |
| Logos testnet 0.4 deployment | Pending | No deployed Program ID or registry PDA recorded |
| Module catalog publication | Pending | Procedure in [`docs/catalog.md`](docs/catalog.md); no catalog URL recorded |
| Adoption and coverage evidence | Pending | Owner checklist below; no public consumer or hosted coverage evidence recorded |

## FURPS Self-Assessment

The criterion-by-criterion assessment is in [`docs/furps.md`](docs/furps.md). It distinguishes implemented source, local test evidence, unrun CI, and operator-controlled deployment and adoption work.

## Limitations and owner actions

- The complete real Storage roundtrip is unverified until `LogoscoreStorage` is aligned with the current Storage module API.
- GitHub Actions has not run for this branch. Nix flake evaluation succeeds, but package builds and interactive Basecamp use are unverified here.
- The testnet 0.4 deployment and registry initialization remain pending.
- Real Geofabrik snapshots hosted in Logos Storage, at least 15 covered countries and 25 verified entries, catalog releases, and five independent consumers (including three Basecamp UI apps) require owner evidence.
- A solution PR to `logos-co/lambda-prize` with the FURPS assessment and adoption evidence remains to be opened.

## Supporting materials

- [Prize specification](docs/LP-0018.md)
- [SDK guide and worked example](docs/sdk.md)
- [FURPS self-assessment](docs/furps.md)
- [Cycle methodology and localnet samples](docs/cycles.md)
- [Module catalog procedure](docs/catalog.md)
- [Gap audit](docs/superpowers/specs/2026-10-29-lp0018-gap-audit.md)
- [CI workflow](.github/workflows/ci.yml)

## Terms and license

By submitting this solution, I confirm that I have read and agree to the [λPrize Terms & Conditions](https://github.com/logos-co/lambda-prize/blob/master/TERMS.md).

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
