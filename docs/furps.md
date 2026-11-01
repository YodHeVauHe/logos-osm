# FURPS self-assessment — LP-0018

This assessment follows the λPrize [solution template](https://github.com/logos-co/lambda-prize/blob/master/solutions/LP-0000.md). It maps requirements to repository evidence and distinguishes locally verified behavior from CI and owner actions. See the [README checklist](../README.md#success-criteria-checklist) for a compact status table.

## Functionality

| Capability | Implementation | Evidence and status |
|---|---|---|
| Discover the frozen 72-region set | `osm_sdk::regions` embeds `regions.json`; optional live index refresh | `osm_sdk/src/regions.rs`; workspace tests pass locally |
| Host one or multiple regions | Fetch, verify Geofabrik MD5, store, then register; bulk registration uses one batch transaction | `osm_sdk/src/host.rs`; unit tests pass locally |
| Import local extracts | Verify MD5 at import time, then store and register | `osm_sdk/src/host.rs`; unit tests pass locally |
| Registry fields and queries | SPEL guest and shared types validate required fields; off-chain query by region, parent, and CID | `osm_registry_core`, `methods/guest`, `osm_sdk/src/registry.rs` |
| Resolve and download | SDK resolves registry entries and selects hosted Storage or Geofabrik fallback paths | `osm_sdk/src/host.rs`; Storage production roundtrip is not verified |
| Consumer SDK/module | Rust API and Basecamp module source expose consumer operations | `docs/sdk.md`, `osm-module`; Nix build and catalog publication remain unverified |
| Basecamp app and consumer example | App source and independent example are present | `osm-app`, `examples/basecamp-consumer`; package build/runtime remains unverified |

**Storage limitation.** `LogoscoreStorage` calls the legacy `uploadFile`, `downloadFile`, and `nameObject` API. The current Storage module exposes `uploadUrl` and `downloadToUrl`. The complete Storage host-query-download roundtrip therefore has not been verified. The in-memory e2e path does not establish Storage interoperability.

**Scope.** The deliverable distributes snapshots. Live OSM sync, map rendering, geocoding, routing, navigation, and custom regions are outside LP-0018 scope. Public testnet deployment is pending.

## Usability

- `osm discover` lists the frozen region set without chain or Storage configuration.
- The CLI exposes discovery, host, import, lookup, update-check, download, and registry initialization commands. `cargo run -p osm_cli --bin osm -- --help` was run locally.
- The Basecamp app source presents region operations, but its package build and interactive runtime are not verified in this documentation run.
- [`docs/sdk.md`](sdk.md) contains a Rust consumer example and a separate Basecamp consumer module example.
- [`docs/catalog.md`](catalog.md) describes catalog release steps; no catalog has been published.

## Reliability

- `Registry` uses `BTreeMap` serialization, and collection query results are ordered by timestamp with region path as a tie-breaker.
- SDK and guest share entry validation. Local unit tests cover validation and registration payload behavior.
- Geofabrik downloads and Storage uploads implement exponential retry delays and report exhausted retries.
- Checksum mismatches fail before registration and remove partial files.
- The in-memory tests cover the host/query/download workflow using test doubles. They do not prove compatibility with the current Logos Storage module API.
- GitHub Actions is green on `main` ([run 41095317888](https://github.com/YodHeVauHe/logos-osm/actions/runs/41095317888)): Rust fmt/clippy/tests on Ubuntu and Windows, the in-memory e2e, the LEZ v0.3.1 sequencer deploy and on-chain e2e, and the Nix module/app builds.

## Performance

The registry is designed to fit below the LEZ 100 KiB account ceiling; the core tests include a serialized-size bound. [`docs/cycles.md`](cycles.md) records observed localnet guest cycle samples for one-entry `register_region` and two-entry `register_batch`, including the measured guest Program ID and toolchain details. These samples are localnet measurements, not universal operation costs. The CI chain job deploys and exercises the registry on a LEZ v0.3.1 sequencer.

## Supportability

- The workspace separates shared types, SDK, CLI, FFI, guest program, Basecamp module, app, and consumer example.
- `cargo build --workspace` and `cargo test --workspace` pass locally. `make idl` regenerates the tracked SPEL IDL. CI repeats them on Ubuntu and Windows.
- `nix flake show` evaluates package outputs locally. The CI Nix job builds `.#osm-install` and `.#osm-app-install` with `logos-module-builder`; the flake reports a QML backend deprecation warning. The repo-level [`module.json`](../module.json) indexes the per-module `metadata.json` manifests the builder consumes, and a test keeps them consistent.
- The CI workflow's Rust, chain, and Nix jobs all pass on `main` ([run 41095317888](https://github.com/YodHeVauHe/logos-osm/actions/runs/41095317888)); the live Geofabrik job runs on manual dispatch only.
- The flake declares Linux and macOS outputs. This does not establish macOS module builds or adoption.
- The repository includes MIT and Apache-2.0 license texts and declares `MIT OR Apache-2.0`.

## Known limitations and pending owner actions

| Item | Evidence status | Remaining work |
|---|---|---|
| Logos testnet 0.4 | No deployment evidence or testnet Program ID recorded | Deploy, test, and record Program ID and registry PDA |
| Logos Storage integration | Adapter targets legacy method names; current module API differs | Align adapter and verify a real host-query-download roundtrip |
| Module catalog | Procedure is documented; no catalog fork or URL recorded | Publish `osm` and `osm-app`, then record `logos-repo.json` URL |
| Adoption | No external consumer or hosted coverage evidence recorded | Provide 15 covered countries, 25 checksum-verifiable entries, five independent consumers (including three Basecamp UI apps), and Linux/macOS use evidence |
| λPrize submission | Draft solution PR open: [logos-co/lambda-prize#187](https://github.com/logos-co/lambda-prize/pull/187) | Update it with testnet 0.4 and adoption evidence, then mark ready |
