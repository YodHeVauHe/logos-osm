# Solution: LP-0018 — OpenStreetMap Integration

**Submitted by:** [YodHeVauHe](https://github.com/YodHeVauHe) · **Prize:** [LP-0018](docs/LP-0018.md) · **License:** [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE)

[![CI workflow badge](https://github.com/YodHeVauHe/logos-osm/actions/workflows/ci.yml/badge.svg)](https://github.com/YodHeVauHe/logos-osm/actions/workflows/ci.yml) · **CI evidence:** GitHub Actions is green on `main` ([run 41095317888](https://github.com/YodHeVauHe/logos-osm/actions/runs/41095317888): Rust on Ubuntu and Windows, LEZ v0.3.1 chain e2e, and Nix module/app builds). A testnet v0.3 practice deployment is recorded under [Program IDs](#program-ids). Both modules are published in the [Logos module catalog](docs/catalog.md) (linux-amd64). The testnet 0.4 deployment and adoption evidence are pending owner actions.

`logos-osm` provides a registry and tools for distributing Geofabrik OpenStreetMap PBF snapshots through Logos. The registry guest targets LEZ v0.3.1's plan/apply API and stores its Borsh registry in a public PDA shard owned by the deployed program account. The repository also includes a Rust SDK, CLI, C FFI, Basecamp core module, and QML app. A practice deployment on public testnet v0.3 is recorded below. The `osm` and `osm-app` modules are published in a Logos module catalog for linux-amd64: `https://raw.githubusercontent.com/YodHeVauHe/logos-osm-modules/main/logos-repo.json`. No testnet 0.4 deployment or ecosystem adoption is claimed.

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
| LEZ v0.3.1 guest (`methods/guest`) | Plans registry shard effects and applies initialization, one-entry upserts, and batches. |
| `osm_registry_core` | Shared registry types, validation, and error codes. |
| `osm_sdk` | Discovery, verification, storage and registry abstractions, hosting, import, queries, downloads, and update checks. |
| `osm_cli` | `osm` command line interface. |
| `ffi` | C ABI used by the Basecamp core module. |
| `osm-module` | Basecamp module exposing SDK operations. |
| `osm-app` | QML distribution app using the module. |
| `examples/basecamp-consumer` | Independent Basecamp consumer example for the `osm` module. |

## Prerequisites

- Rust 1.98.1 and Cargo for host workspace and CLI builds.
- Nix with flakes enabled for Basecamp module and app packages.
- RISC Zero `rzup` Rust 1.97.0 / C++ 2024.1.5 toolchains for `make build`; Docker is not used by this build. The build is toolchain pinned but not Docker deterministic, and host dependency resolution may change without lockfiles.
- A LEZ v0.3.1 `wallet` binary and wallet home for deployment and transactions; the SDK CLI uses `LEE_WALLET_HOME_DIR` and `OSM_RPC_URL`.
- `logosctl` 0.3.2 with the current `storage_module` API for production Storage operations. The in-memory mode is for development and tests.

## Build and test

Host commands verified on this branch:

```sh
RUSTUP_TOOLCHAIN=1.98.1 cargo test --workspace
RUSTUP_TOOLCHAIN=1.98.1 make idl
RUSTUP_TOOLCHAIN=1.98.1 make build
RUSTUP_TOOLCHAIN=1.98.1 cargo test -p osm_e2e --features e2e
```

`make build` uses `risc0-build::embed_methods` with the installed rzup toolchains, wraps the ELF in the ProgramBinary format required by v0.3.1 `wallet program-loader`, and prints the image ID. `make idl` regenerates the tracked `idl/osm-registry.idl.json`.

Nix builds are not claimed here; run the package builds if Nix is installed.

Run the e2e suite with `RUSTUP_TOOLCHAIN=1.98.1 cargo test -p osm_e2e --features e2e`. `localnet_registry_host_query` uses a real sequencer only when `LEZ_LOCALNET=1` and the LEZ v0.3.1 registry CLI, deployed program account, wallet home, and fee payer are configured. A real Storage upload-download roundtrip now passes through `LogosctlStorage`; evidence covers one local node on `logos.test`, not a multi-node network transfer.

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

`discover` lists the frozen set without a registry connection. Registry and hosting commands need the deployed program header account ID, a configured wallet home and fee payer, and (optionally) `OSM_RPC_URL`; hosting also needs a working Logos Storage setup. The instruction CLI accepts `--rpc-url`, `--program-id`, `--payer`, JSON Borsh values, and has `get` / `pda` commands. See [`docs/sdk.md`](docs/sdk.md) for SDK configuration and API details. `--storage-memory` selects test doubles and is not a publishing mode.

### Basecamp app

Build the FFI library with `make ffi`, then build the app package with `nix build .#osm-app-install`. Install and launch the resulting app through Basecamp; the app provides region discovery, hosting, import, download, and update-check controls. The package build and interactive runtime have not been verified on this branch.

## Program IDs

**Logos testnet 0.4:** pending operator deployment. LP-0018 requires the 0.4 deployment; none has occurred.

### Testnet v0.3 practice deployment (not the 0.4 deployment LP-0018 requires)

These IDs are on the **public Logos testnet v0.3** LEZ zone (`https://testnet.lez.logos.co`). They are not testnet 0.4 IDs and do not satisfy the LP-0018 testnet 0.4 criterion. The deployment was made on 2026-10-30 with the LEZ v0.3.1 `wallet program-loader`, using an immutable header and fresh segment accounts.

| Item | Value |
|---|---|
| Program header account ID (`--program-id` / `OSM_PROGRAM_ID`) | [`C31bCPXKa5uy1hzmegqMABWLbavoSJbY4Rhrc5yNXPrY`](https://explorer.testnet.lez.logos.co/account/C31bCPXKa5uy1hzmegqMABWLbavoSJbY4Rhrc5yNXPrY) |
| Guest image ID (stored in the header) | `2e27ae50d12ae9e823c6ac7659a3fa625b4a30fab5ad8d844ff25707adec36b6` |
| Registry PDA (seed `registry`) | [`C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS`](https://explorer.testnet.lez.logos.co/account/C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS) |
| Segment 0 | [`WTeQHg8pbWP8urBxS3k74Fefpv32CejdrjcVjvh9CjG`](https://explorer.testnet.lez.logos.co/account/WTeQHg8pbWP8urBxS3k74Fefpv32CejdrjcVjvh9CjG) |
| Segment 1 | [`DbzHoDkddG7ge4RaNBmB4cw3r7t8SHXu6v4uzvmQEDg3`](https://explorer.testnet.lez.logos.co/account/DbzHoDkddG7ge4RaNBmB4cw3r7t8SHXu6v4uzvmQEDg3) |
| Segment 2 | [`4u7qoMKYxhGkvbTcA3S149NN2ybhoWtG1VWq1Lw22wpR`](https://explorer.testnet.lez.logos.co/account/4u7qoMKYxhGkvbTcA3S149NN2ybhoWtG1VWq1Lw22wpR) |
| Segment 3 | [`9SjWzxmQ7SvBRmda2beV9ct6o6mABwxfzHGjrJr1c2va`](https://explorer.testnet.lez.logos.co/account/9SjWzxmQ7SvBRmda2beV9ct6o6mABwxfzHGjrJr1c2va) |
| Fee payer | `59VSwCJemEPWXBFGwQ8gzyZroNZ4AuQ81hJwpGUqMoEL` |

| Transaction | Block | Hash |
|---|---|---|
| Write segment 3 | 69720 | [`138dac7bfd599bcc993e4f9ed9e3a21f856b471e02d523b223b62b7b59d78f6e`](https://explorer.testnet.lez.logos.co/transaction/138dac7bfd599bcc993e4f9ed9e3a21f856b471e02d523b223b62b7b59d78f6e) |
| Write segment 2 | 69721 | [`3e87e8bcc560c96e3507272f0016df2a82b2c599d664cfa399c4d2a8da9123ad`](https://explorer.testnet.lez.logos.co/transaction/3e87e8bcc560c96e3507272f0016df2a82b2c599d664cfa399c4d2a8da9123ad) |
| Write segment 1 | 69722 | [`8873b6563286130fb350f33003f3385681faeba8caf8415c50cd37e29fe6573e`](https://explorer.testnet.lez.logos.co/transaction/8873b6563286130fb350f33003f3385681faeba8caf8415c50cd37e29fe6573e) |
| Write segment 0 | 69725 | [`f694c7e448dc50322ccdbe6f13db8a3376b672339e33243e0ccba2e557c846bb`](https://explorer.testnet.lez.logos.co/transaction/f694c7e448dc50322ccdbe6f13db8a3376b672339e33243e0ccba2e557c846bb) |
| Create immutable header | 69726 | [`bcdc3f7ecb8b5d66fb920f1a821f29e8a8b2af4d25a091f5569c73c5efb12c83`](https://explorer.testnet.lez.logos.co/transaction/bcdc3f7ecb8b5d66fb920f1a821f29e8a8b2af4d25a091f5569c73c5efb12c83) |
| Smoke test: `InitRegistry` | 69735 | [`4df3a67cd07905a5155c97bbc9478d358bef737ef5a2310d51527358697b104a`](https://explorer.testnet.lez.logos.co/transaction/4df3a67cd07905a5155c97bbc9478d358bef737ef5a2310d51527358697b104a) |
| Smoke test: `RegisterRegion` (`kenya`) | 69743 | [`6f9ffdbdf48f6089fb90c2a51f8376f9fbae47397c7179c48aee6f656897425c`](https://explorer.testnet.lez.logos.co/transaction/6f9ffdbdf48f6089fb90c2a51f8376f9fbae47397c7179c48aee6f656897425c) |

All seven hashes and block heights were checked against the sequencer with `wallet chain-info transaction --hash <hash>`. The header account data begins with the image ID above. When the IDs were recorded, the explorer's indexer was several hundred blocks behind the sequencer, so the explorer links may not resolve until it catches up.

The smoke test registered `kenya` (country level) with source `https://download.geofabrik.de/africa/kenya-latest.osm.pbf`, Geofabrik MD5 `39f15a2f62a406f50e153614d7db31e6`, and version `261028`. `osm_registry_cli get` read the entry back from the registry PDA. The entry has `hosted: false` and the placeholder CID `unhosted:geofabrik-fallback` because no bytes were uploaded to Logos Storage, so it does **not** count toward the hosted coverage criterion.

```bash
OSM_PROGRAM_ID=C31bCPXKa5uy1hzmegqMABWLbavoSJbY4Rhrc5yNXPrY \
LEE_WALLET_HOME_DIR=<wallet home> \
examples/target/debug/osm_registry_cli get
```

The image ID depends on the build environment. Earlier README revisions listed `c06bd5aab872896a7da067e7694e3a3ddda862fb442ce0552b9b50101ad4b65f`, which came from a different local build. The deployed program is the `make build` output of commit `913cacf`, with image ID `2e27ae50…36b6`.

## Regions

The closed, non-overlapping set contains 72 entries: 48 country-level extracts and 24 subregions for the United States, India, China, and Russia. [`regions.json`](regions.json) is the source of truth; the prize specification lists the regions by name in [Predefined Regions](docs/LP-0018.md#predefined-regions-closed-set). No custom regions are included.

## Success Criteria Checklist

Status is based on repository artifacts, local commands, and the green `main` CI run ([run 41095317888](https://github.com/YodHeVauHe/logos-osm/actions/runs/41095317888)). A checked implementation item does not imply a public testnet 0.4 deployment.

| Requirement | Status | Evidence |
|---|---|---|
| LEZ v0.3.1 plan/apply registry with required fields and batch registration | Guest host tests, workspace tests, and local sequencer e2e pass | [`methods/guest`](methods/guest), [`osm_registry_core`](osm_registry_core) |
| Frozen region discovery, checksums, hosting/import, queries and updates | Implemented; Rust workspace tests pass | [`regions.json`](regions.json), [`osm_sdk`](osm_sdk), [`osm_cli`](osm_cli) |
| CLI and consumer SDK | Implemented; CLI help and workspace build verified | [`osm_cli`](osm_cli), [`docs/sdk.md`](docs/sdk.md) |
| Basecamp core module, app, and independent consumer example | `osm`, `osm-app`, `osm_consumer_example` and the `osm-consumer-app` UI example build with `logos-module-builder` in the CI Nix job; repo-level [`module.json`](module.json) indexes the per-module `metadata.json` manifests. In portable Basecamp (Linux x86_64), `osm-consumer-app` resolved `china/henan` and downloaded a checksum-verified snapshot through `osm_consumer_example` → `osm` | [`osm-module`](osm-module), [`osm-app`](osm-app), [`examples/basecamp-consumer`](examples/basecamp-consumer), [`examples/basecamp-consumer-app`](examples/basecamp-consumer-app), [embedding example](docs/sdk.md#embedding-example-a-basecamp-ui-app-on-top-of-the-consumer), [`flake.nix`](flake.nix) |
| LEZ v0.3.1 instruction/account IDL generated and tracked | `make idl` regenerates the Borsh plan/apply contract; the CI IDL freshness check passes. Not SPEL-generated: SPEL still targets LEZ v0.2.5-rc2 | [`idl/osm-registry.idl.json`](idl/osm-registry.idl.json), [`docs/idl.md`](docs/idl.md) |
| In-memory e2e | `cargo test -p osm_e2e --features e2e` passes in CI | [`tests/e2e`](tests/e2e) |
| Real LEZ v0.3.1 sequencer e2e and cycle capture | Passes locally and in the CI chain job; cycle sample recorded | [CI workflow](.github/workflows/ci.yml), [`docs/cycles.md`](docs/cycles.md) |
| Logos Storage host-query-download roundtrip | Storage upload-download passed against one local `storage_module` 3.0.2 node via logosctl 0.3.2; 3,507,206-byte Andorra PBF reproduced with matching MD5 and SHA-256. Registry query and multi-node transfer are not part of this result. | [`osm_sdk/src/storage.rs`](osm_sdk/src/storage.rs), [SDK storage configuration](docs/sdk.md) |
| Logos testnet v0.3 practice deployment (not 0.4) | Deployed; init and one-entry register/read-back smoke test passed on the public testnet | [Program IDs](#testnet-v03-practice-deployment-not-the-04-deployment-lp-0018-requires) |
| Logos testnet 0.4 deployment | Pending operator action | No testnet 0.4 program header account ID or registry PDA recorded |
| Module catalog publication | Done (linux-amd64) | [`logos-repo.json`](https://raw.githubusercontent.com/YodHeVauHe/logos-osm-modules/main/logos-repo.json); details in [`docs/catalog.md`](docs/catalog.md) |
| Adoption and coverage evidence | Pending | Owner checklist below; no public consumer or hosted coverage evidence recorded |

## FURPS Self-Assessment

The criterion-by-criterion assessment is in [`docs/furps.md`](docs/furps.md). It distinguishes implemented source, local test evidence, CI results, and operator-controlled deployment and adoption work.

## Limitations and owner actions

- Storage roundtrip evidence is a single-node local upload-download only; no multi-node transfer or registry query was exercised in that roundtrip.
- GitHub Actions is green on `main` ([run 41095317888](https://github.com/YodHeVauHe/logos-osm/actions/runs/41095317888)), including the Nix builds of the `osm` module and `osm-app`. Interactive Basecamp use is not yet verified.
- The testnet v0.3 practice deployment is complete. The required testnet 0.4 deployment and registry initialization remain pending.
- Real Geofabrik snapshots hosted in Logos Storage, at least 15 covered countries and 25 verified entries, and five independent consumers (including three Basecamp UI apps) require owner evidence.
- A draft solution PR is open at [logos-co/lambda-prize#187](https://github.com/logos-co/lambda-prize/pull/187); it will be updated with the testnet 0.4 deployment and adoption evidence.

## Supporting materials

- [Prize specification](docs/LP-0018.md)
- [SDK guide and worked example](docs/sdk.md)
- [FURPS self-assessment](docs/furps.md)
- [Cycle methodology and localnet samples](docs/cycles.md)
- [Module catalog](docs/catalog.md)
- [Gap audit](docs/superpowers/specs/2026-10-29-lp0018-gap-audit.md)
- [CI workflow](.github/workflows/ci.yml)

## Terms and license

By submitting this solution, I confirm that I have read and agree to the [λPrize Terms & Conditions](https://github.com/logos-co/lambda-prize/blob/master/TERMS.md).

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
