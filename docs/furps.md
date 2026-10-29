# FURPS self-assessment — LP-0018

This assessment follows the λPrize
[solution template](https://github.com/logos-co/lambda-prize/blob/master/solutions/LP-0000.md).
It maps each FURPS dimension to the artefacts that satisfy it and separates
what is implemented from what is still an operator step. The criterion-by-
criterion checklist is in the [README](../README.md#success-criteria-checklist).

## Functionality

| Capability | Implementation | Evidence |
|---|---|---|
| Region discovery over the frozen 72-region set | `osm_sdk::regions` embeds `regions.json`; optional live URL refresh from the Geofabrik index | `osm_sdk/src/regions.rs`, `osm discover [--refresh]` |
| Host workflow (fetch → verify → store → register) | `OsmSdk::host_region` | `osm_sdk/src/host.rs`, unit tests with a wiremock Geofabrik |
| Bulk host with per-region opt-out and one batch transaction | `OsmSdk::host_regions`, `register_batch` | `host_regions_batches_into_one_tx`, CLI `--exclude` |
| Local import with MD5 verification at import time | `OsmSdk::import_local` (optional offline checksum override) | `import_local_uses_override_checksum`, CLI `--md5` |
| Update check tracked per region path | `OsmSdk::check_updates` | `check_updates_detects_newer_central` |
| Download from Logos Storage with Geofabrik fallback | `OsmSdk::download_region` | `download_prefers_storage_then_falls_back` |
| On-chain registry with required fields, unique region key, explicit parent/level | `osm_registry_core::RegionEntry` + guest validation | `osm_registry_core/src/lib.rs`, `methods/guest` |
| Query by region, parent and CID | Off-chain borsh decode + filters | `osm_sdk/src/registry.rs` |
| Batch registration | `register_batch` with hex(borsh(Vec<RegionEntry>)) | Guest instruction, `batch_payload_roundtrip` |
| Consumer SDK/module separate from the distribution app | `osm_sdk` full API; `osm-module` consumer subset over FFI | `docs/sdk.md`, `osm-module/metadata.json` |
| End-to-end pipeline | In-memory CI suite and a local-sequencer chain job | `tests/e2e/src/lib.rs`, `.github/workflows/ci.yml` |

**Limitations.** The deliverable distributes snapshots only: no live OSM sync,
map rendering, geocoding, routing, navigation or custom regions. Those are out
of scope in LP-0018. The public testnet 0.4 deployment is not yet recorded.

## Usability

- `osm discover` works with no chain, storage or network configuration, which
  gives a first-run path that needs only Rust.
- The CLI prints hosted status, version, CID and verification result, and every
  query/report command supports `--json` for scripting.
- `--storage-memory` runs the full pipeline without `logoscore` or a sequencer
  (mock backends), so the SDK can be evaluated before infrastructure is set up.
- The Basecamp app presents the region set with level grouping, hosted/version
  columns, a verification line, bulk selection with opt-out, local import,
  download and an update banner.
- [`docs/sdk.md`](sdk.md) contains the worked "resolve a region to its CID and
  metadata" example that most consumer modules will follow.
- [`docs/catalog.md`](catalog.md) documents installation through the Basecamp
  package manager once a catalog is published.

The onboarding gap is operational: a real host/import needs a configured LEZ
node, signer, `logoscore` and `storage_module`. The README lists those
prerequisites and the exact commands.

## Reliability

- **Deterministic state.** `Registry` uses `BTreeMap`, so borsh bytes do not
  depend on insertion order. The guest rejects writes that would exceed the
  LEZ 100 KiB account ceiling.
- **Validation in two places.** The SDK runs the same `validate_entry` rules as
  the guest before submitting, so malformed entries fail locally instead of
  consuming a transaction.
- **Retry policy.** Geofabrik downloads and Logos Storage uploads share
  `backoff_delays` (1s, 2s, 4s … capped at 30s) and return
  `SdkError::RetriesExhausted` with the underlying error after the last try.
- **Fail-closed checksums.** A mismatch returns
  `ChecksumMismatch { region, expected, actual }`, removes the partial file and
  never registers anything. The CLI maps this to exit code 2.
- **Partial bulk failure.** A region that fails download/verification is
  reported in `HostReport::skipped`; the remaining verified regions are still
  registered in one batch.
- **No hidden dependency.** The only external endpoints are the configured
  Logos node/sequencer, Logos Storage and Geofabrik (or a mirror via
  `--geofabrik-base`). Hosted downloads do not need Geofabrik. There is no
  analytics or optional third-party service.

## Performance

- The full 72-entry registry occupies roughly 19 KB, well below the 100 KiB
  account limit; a bulk registration is a single transaction.
- The guest logs cycle counts around `init_registry`, `register_region` and
  `register_batch`, and the chain CI job greps those lines.
- `docs/cycles.md` documents the measurement method and the expected linear
  scaling of batch registration. The measured values are still placeholders
  produced by CI; they are not presented as results until captured on a green
  chain run.

No throughput or latency claims are made beyond the design properties above.

## Supportability

- **Structure.** The workspace separates shared types (`osm_registry_core`),
  the SDK (`osm_sdk`), the CLI (`osm_cli`), the C ABI (`ffi`), the guest
  (`methods/guest`), the Basecamp module (`osm-module`) and the app
  (`osm-app`). The guest stays thin because validation lives in the shared
  crate.
- **Tests.** Unit and integration tests cover validation, Geofabrik parsing,
  versions, storage, registry payloads, workflows and the e2e pipeline. The
  e2e suite runs in memory on every push and against a local sequencer in the
  chain job.
- **CI.** `rust` (Ubuntu + Windows) runs fmt, clippy, tests and e2e; `chain`
  builds the guest deterministically, generates the IDL, starts `lgs localnet`,
  deploys, runs the on-chain e2e and captures cycle logs; `nix` builds the
  module and app; `live-geofabrik` is dispatch-only.
- **Toolchain pinning.** `spel-framework` and `lee_core` are pinned, and the
  known-good framework is vendored, so the Program ID is reproducible.
- **Documentation.** README, SDK guide, catalog procedure, cycle methodology
  and this assessment live in-repo; the prize specification and implementation
  plan are kept for reference.
- **Licensing.** `MIT OR Apache-2.0` with both license texts committed.
- **Platforms.** Host Rust is tested on Ubuntu and Windows. Modules are built on
  Linux; the flake declares Linux/macOS outputs, but macOS module builds and
  Android are not yet validated in CI.

## Known limitations and pending operator steps

| Item | Current state | What remains |
|---|---|---|
| Testnet 0.4 | Local-sequencer deployment is in CI | Deploy to the public zone and record the Program ID and registry PDA |
| IDL | `make idl` generates the tracked artifact; CI fails if it is stale and uploads it for inspection | Keep regeneration deterministic as the guest interface changes |
| Cycle counts | Guest instrumentation + CI capture are wired | Commit measured values in `docs/cycles.md` |
| Module catalog | Fork/release procedure documented | Publish the catalog and record its `logos-repo.json` URL |
| Adoption | No evidence claimed | 15 countries / 25 verified entries, 5 independent modules (3 Basecamp UI apps), macOS coverage |

These gaps are deployment and adoption work, not missing implementation of the
success criteria.
