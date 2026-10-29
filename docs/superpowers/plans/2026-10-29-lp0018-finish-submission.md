# LP-0018 Submission Finish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close every class A/B repository gap in the LP-0018 audit, restore green CI, and leave only explicit owner-controlled deployment, publication and adoption work.

**Architecture:** Keep the existing SDK as the source of workflow behavior. Repair the generator/build inputs first, enforce the generated artifact in CI, then make the on-chain e2e job use real configured backends and make the QML bridge render returned registry data. Finish with timestamp ordering, measured cycle evidence and factual docs.

**Tech Stack:** Rust stable, Cargo, SPEL framework, RISC0/rzup, lgs, logoscore/Logos Storage, Qt Remote Objects/repc, logos-module-builder, Nix, GitHub Actions.

**Spec:** `docs/LP-0018.md`; audit: `docs/superpowers/specs/2026-10-29-lp0018-gap-audit.md`.

## Global Constraints

- Registry minimum fields are `region`, `parent`, `level`, `cid`, `source_url`, `checksum`, `version`, `hosted`, and `timestamp`.
- The predefined set is closed and non-overlapping; do not add custom regions or levels.
- Verify Geofabrik MD5 at import time; use the content-addressed CID as the hosted-download integrity anchor.
- Bulk registration supports multiple regions in one batch transaction.
- No mandatory third-party service beyond the configured sequencer/LEZ node, Logos Storage or other Logos protocols, and Geofabrik for index, import-time checksum, and unhosted fallback.
- Upload retries use exponential backoff and report a clear exhausted-retries error.
- CI on the default branch must be green; chain e2e covers host → query → download against a real sequencer in standalone mode.
- Generate and commit the registry IDL through SPEL.
- Build both Basecamp artifacts with `logos-module-builder` (`mkLogosModule` / QML builder).
- Document the cycle count method for register and batch-register; report only measured values.

## File Map

- `examples/src/bin/generate_idl.rs`: path passed to the SPEL IDL macro.
- `idl/osm-registry.idl.json`: generated, tracked registry IDL.
- `.github/workflows/ci.yml`: fail-closed IDL check, actual localnet/Storage e2e configuration, module build, cycle artifact handling.
- `tests/e2e/Cargo.toml`, `tests/e2e/src/lib.rs`: in-memory e2e and real-backend localnet e2e selection.
- `osm-app/src/osm_app.rep`, `osm-app/src/osm_app_plugin.cpp`, `osm-app/src/osm_app_plugin.h`, `osm-app/src/qml/Main.qml`: repc-compatible slots and decoded region/entry/update state.
- `osm_sdk/src/registry.rs`: timestamp-sorted results for public query collections.
- `osm_sdk/src/registry.rs` tests: pin timestamp order.
- `docs/cycles.md`: measured operation values plus reproducible measurement method.
- `README.md`, `docs/furps.md`: claims aligned to actual CI/deployment evidence.
- `docs/sdk.md`, `examples/`: minimal independent Basecamp consumer module example.
- `tools/check_submission_docs.py`: focused assertions against stale documentation claims and missing tracked artifacts.

---

## Priority 0 — Restore reported CI failures

### Task 1: Make IDL generator resolve the guest source

**Files:**
- Modify: `examples/src/bin/generate_idl.rs`
- Generate: `idl/osm-registry.idl.json`

**Interfaces:** Keep the existing SPEL `generate_idl!` macro and point it at `methods/guest/src/bin/osm_registry.rs` relative to the examples crate root. Preserve stdout as the generated JSON stream consumed by `make idl`.

- [ ] **Step 1: Reproduce the failing generation command**

Run: `cargo run --manifest-path examples/Cargo.toml --bin generate_idl`

Expected before the fix: failure to read the reported `src/bin/../methods/guest/src/bin/osm_registry.rs` path.

- [ ] **Step 2: Correct the macro input path**

Change the macro path to `../../../methods/guest/src/bin/osm_registry.rs`, relative to `examples/src/bin` as used by this macro invocation.

- [ ] **Step 3: Generate the tracked IDL**

Run: `make idl`

Expected: `idl/osm-registry.idl.json` is created as valid JSON and contains `init_registry`, `register_region`, and `register_batch` instructions.

- [ ] **Step 4: Check the artifact and command status**

Run: `python3 -c 'import json; d=json.load(open("idl/osm-registry.idl.json")); print(sorted(i["name"] for i in d["instructions"]))'`

Expected: successful parse and the three instruction names. If the generated IDL schema uses a different top-level key, inspect the generated JSON and assert the same three names at its actual documented structure.

Verification environment: Linux Cargo/toolchain; no Nix or Docker required for this generator path.

### Task 2: Diagnose and repair the repc parser failure

**Files:**
- Modify: `osm-app/src/osm_app.rep`
- Modify if required by repc-generated API: `osm-app/src/osm_app_plugin.h`, `osm-app/src/osm_app_plugin.cpp`

**Interfaces:** Preserve the app bridge operations: host selected regions, import a region, download a region, refresh, and check updates. The audit has not confirmed whether the unsupported token is the three-argument slot, a preceding comment, or other grammar.

- [ ] **Step 1: Reproduce the reported failing app build**

Run in Nix-enabled CI: `nix build .#osm-app-install --show-trace`

Expected before the fix: repc reports `Unknown token encountered` at or near `osm-app/src/osm_app.rep:25`.

- [ ] **Step 2: Reduce the input to identify the rejected construct**

In a temporary worktree or uncommitted edit, reduce the `.rep` file to the class, properties, and one slot; restore slots incrementally until repc fails. Record whether comments, argument names, or the three-argument signature trigger the failure. Do not commit diagnostic reductions.

- [ ] **Step 3: Add a minimal failing repc fixture or pin the minimal failing declaration**

Keep a small `.rep` fixture only if the repository has a maintainable repc fixture mechanism; otherwise use the `nix build` failure above as the red check and document the triggering declaration in the commit.

- [ ] **Step 4: Apply the smallest syntax-compatible interface change**

Keep the same semantic argument values and update the generated slot declaration/implementation signatures together if repc requires a different signature shape.

- [ ] **Step 5: Build both Basecamp outputs**

Run in Nix CI: `nix build .#osm-install .#osm-app-install`

Expected: both package derivations complete successfully. Run the same command on a Nix-enabled Linux machine when available.

Verification environment: Nix + logos-module-builder. This cannot be verified on the current Linux box because Nix is unavailable.

## Priority 1 — Make CI gates prove the required behavior

### Task 3: Commit the IDL and make freshness fail closed

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`
- Modify: `docs/furps.md`
- Use: `idl/osm-registry.idl.json` from Task 1

**Interfaces:** `make idl` must deterministically rewrite the tracked IDL. The chain job fails if the committed IDL differs from generated output.

- [ ] **Step 1: Verify current workflow accepts stale/missing IDL**

Inspect the `IDL freshness` step; its `|| echo ::warning::` makes a diff non-fatal. Pin the intended behavior in CI by making freshness required.

- [ ] **Step 2: Replace warning-only freshness with a failing check**

Run `make idl`, then `git diff --exit-code -- idl/osm-registry.idl.json` in the chain workflow. Remove the warning fallback. Retain artifact upload only after the check passes.

- [ ] **Step 3: Update documentation claims**

Replace statements that the IDL is absent or warning-only with statements that it is tracked and CI-enforced. Do not claim the workflow is green until a successful run exists.

- [ ] **Step 4: Verify locally**

Run: `make idl && git diff --exit-code -- idl/osm-registry.idl.json`

Expected: a clean diff after the generated artifact is committed; changing the artifact locally makes the check fail.

### Task 4: Run the chain e2e against an actual sequencer and Storage backend

**Files:**
- Modify: `tests/e2e/Cargo.toml`
- Modify: `tests/e2e/src/lib.rs`
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md` (prerequisites/configuration only)

**Interfaces:** Keep the deterministic in-memory test as a separate path. For the chain job, construct `SpelCliRegistryClient` using `OSM_PROGRAM_ID`, generated IDL and lgs wallet configuration; configure the real Logos Storage backend through `logoscore`/`storage_module`. The Geofabrik file source can remain a local wiremock fixture so the e2e does not depend on a public download service. Assert the registry query reads back the submitted entry and that downloading the resulting CID returns the hosted bytes.

- [ ] **Step 1: Pin the missing real-backend behavior with a failing test**

Add a test selected only when `LGS_LOCALNET=1` that asserts real backend configuration is required and that the test obtains the registry entry through `SpelCliRegistryClient`. Remove the current environment branch that only prints a message. Keep `host_query_download_roundtrip` for in-memory CI.

- [ ] **Step 2: Run the local-only test and confirm it fails before configuration**

Run: `cargo test -p osm_e2e --features e2e localnet_host_query_download -- --nocapture`

Expected before wiring: explicit failure for missing localnet/backend configuration, never a passing mock-only run.

- [ ] **Step 3: Wire the real registry and Storage implementations**

Use the existing `SpelCliRegistryClient` and `LogoscoreStorage` from `osm_sdk/src/registry.rs` and `osm_sdk/src/storage.rs`. Keep the wiremock source and deterministic PBF bytes. Ensure the storage backend returns an actual CID and download retrieves by that CID.

- [ ] **Step 4: Provision the chain job**

Update `.github/workflows/ci.yml` to start localnet, deploy the built program, initialize/fund its signer, make the Logos Storage module and `logoscore` available, set all required program/IDL/wallet/storage variables, and invoke the localnet-only test. Fail if required setup is absent.

- [ ] **Step 5: Verify both e2e modes**

Run in Rust CI: `cargo test -p osm_e2e --features e2e`

Run in chain CI: `LGS_LOCALNET=1 OSM_PROGRAM_ID=<built-id> cargo test -p osm_e2e --features e2e localnet_host_query_download -- --nocapture`

Expected: in-memory mode passes without Logos daemons; chain mode logs successful real registry submission/readback and Logos Storage put/get by CID. The chain command must not select `MockRegistryClient` or `InMemoryStorage`.

Verification environment: local test is runnable on this box; real sequence/Storage verification is CI/localnet infrastructure work and belongs to class B.

### Task 5: Make CI gates exercise all reported jobs and preserve real evidence

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `docs/cycles.md`

**Interfaces:** Default branch jobs must fail on generator, chain e2e, and both Nix module builds. Cycle results must be downloadable and also committed or otherwise retained in a reviewable repository artifact; CI must never write placeholder numbers as measurements.

- [ ] **Step 1: Add explicit gate assertions**

Keep `make build`, `make idl`, IDL freshness, localnet e2e, `nix build .#osm-install`, and `nix build .#osm-app-install` as required chain/Nix steps. Remove `|| true` from required setup and measurement commands; only tolerate log retrieval failures when the workflow then fails an explicit “measurements found” assertion.

- [ ] **Step 2: Run the full workflow on a pull request**

Expected: all `rust`, `chain`, and `nix` jobs pass; no IDL warning substitutes for a required artifact. Keep the dispatch-only Geofabrik smoke separate.

Verification: GitHub Actions on Linux/Nix/Docker; not available as a full local command on this box.

## Priority 2 — Repair app state and remaining code-level FURPS gaps

### Task 6: Populate QML state from module JSON responses

**Files:**
- Modify: `osm-app/src/osm_app_plugin.cpp`
- Modify: `osm-app/src/osm_app_plugin.h` if helper declarations are needed
- Modify: `osm-app/src/qml/Main.qml`
- Test: focused parser tests under the app's existing Qt test setup, or a small pure C++/Qt test target if none exists

**Interfaces:** Module methods return JSON strings. `discover` returns a JSON array; `query` and `checkUpdates` return JSON collections. `registryEntries` must be a JSON array consumed into `entryMap[region]`. QML actions must display hosted/version and verified status from actual results.

- [ ] **Step 1: Add failing JSON decoding tests**

Test a region array, an entry array, an update array, and an `{ok:false,error:...}` result. Assert arrays remain arrays and errors surface as error text rather than silently becoming empty objects.

- [ ] **Step 2: Run the focused tests**

Run the app Qt test target in Nix CI. Expected before implementation: region/entry/update result-shape assertions fail.

- [ ] **Step 3: Decode JSON by result shape and load registry entries**

Parse a module response to `QJsonDocument` and preserve array/object distinctions. On refresh, load predefined regions and query/resolve entries for them, then serialize entry objects to `registryEntries`. On update, serialize the returned update array. Surface module errors in `status`.

- [ ] **Step 4: Build and exercise the view**

Run: `nix build .#osm-app-install`

Expected: QML binds each region to its registry record, shows hosted/not-hosted and version correctly, and update/verification results remain visible after action completion.

Verification environment: Qt/repc and Nix CI (class B).

### Task 7: Ensure timestamp order for registry query results

**Files:**
- Modify: `osm_sdk/src/registry.rs`
- Modify: `osm_sdk/src/registry.rs` unit tests

**Interfaces:** Keep the `RegistryClient` storage map keyed by region for deterministic serialization. Return all collection query results in ascending `timestamp` order, with region path as a deterministic tie-breaker; preserve single-region lookup semantics.

- [ ] **Step 1: Add failing tests for all collection query helpers**

Add shuffled entries and assert `query_by_parent`, `query_by_cid`, and any exposed all-entry/list helper return ascending timestamp then region. Include equal timestamps and empty results.

- [ ] **Step 2: Run the focused tests**

Run: `cargo test -p osm_sdk registry::tests`

Expected before fix: the parent query passes but CID/all-entry ordering test fails.

- [ ] **Step 3: Apply stable common ordering**

Sort collection query results with key `(entry.timestamp, entry.region.as_str())`; do not change BTreeMap serialization or on-chain storage.

- [ ] **Step 4: Verify**

Run: `cargo test -p osm_sdk registry::tests`

Expected: every collection ordering test passes.

### Task 8: Add a minimal Basecamp consumer embedding example

**Files:**
- Create: `examples/basecamp-consumer/metadata.json`
- Create: `examples/basecamp-consumer/CMakeLists.txt`
- Create: `examples/basecamp-consumer/src/consumer_impl.h`
- Create: `examples/basecamp-consumer/src/consumer_impl.cpp`
- Modify: `docs/sdk.md`
- Modify: `flake.nix` only if needed to expose the example derivation

**Interfaces:** The example depends on the standalone `osm` module only and calls `discover`, `resolve`, and `downloadRegion`; it must not depend on `osm-app`. It demonstrates a consumer retrieving a region CID/metadata using the published module interface.

- [ ] **Step 1: Add a failing package/build check**

Add the example to the Nix outputs and CI target before implementing its module calls. Expected: the package target is absent or fails to build.

- [ ] **Step 2: Implement the minimal consumer module**

Use the existing Basecamp module interface and call only the documented consumer API. Keep UI out of scope; a module that exposes a resolve method and returns CID/metadata is sufficient.

- [ ] **Step 3: Document the exact build/install/use sequence**

Update `docs/sdk.md` with the example path, dependency declaration, invoked module methods, and `nix build` command.

- [ ] **Step 4: Verify**

Run in CI: `nix build .#osm-consumer-example`

Expected: the independent consumer builds with `osm` dependency and without `osm-app`.

Verification environment: Nix/Basecamp build tools (class B).

## Priority 3 — Produce measured evidence and correct claims

### Task 9: Capture reproducible cycle counts and keep measurements reviewable

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `docs/cycles.md`
- Modify: `methods/guest/src/bin/osm_registry.rs` only if the logged fields need to include batch size/operation labels

**Interfaces:** Record actual cycle counts for one `register_region` and `register_batch` with declared batch size. Keep the invocation/how-to-use `cycle_bench` instructions. Do not replace missing measurements with placeholder text.

- [ ] **Step 1: Add a CI assertion for both measured operations**

After the real chain e2e, parse sequencer/guest output and fail unless one `register_region` and one `register_batch` cycle result are present. Include batch size in the emitted record.

- [ ] **Step 2: Run the chain benchmark job**

Run the chain workflow with `cargo risczero`/localnet. Expected before the assertion is satisfied: job fails when either operation log is missing.

- [ ] **Step 3: Store actual output in the repository**

Update `docs/cycles.md` with captured numeric values, guest/toolchain version, and batch size. Retain a CI artifact as a copy, not as the sole source. Keep the instructions for reproducing with LEZ `cycle_bench`.

- [ ] **Step 4: Review measurement reproducibility**

Run: `rg -n 'filled by|placeholder|[0-9]+ cycles' docs/cycles.md`

Expected: no placeholder wording and measured entries identify their operation/configuration.

Verification environment: RISC0/Docker/LEZ localnet CI (class B).

### Task 10: Reconcile README and FURPS claims with current evidence

**Files:**
- Modify: `README.md`
- Modify: `docs/furps.md`
- Modify: `docs/catalog.md` only if publication-status wording needs adjustment
- Create: `tools/check_submission_docs.py`

**Interfaces:** Documentation must distinguish source implementation, locally tested behavior, CI-verified behavior, and owner-operated deployment/adoption evidence. Preserve an explicit unchecked status until the matching evidence exists.

- [ ] **Step 1: Add a failing documentation consistency check**

Write `tools/check_submission_docs.py` using Python's `pathlib` and `re`. Assert that `idl/osm-registry.idl.json` exists and parses as JSON; assert the README/FURPS no longer contain stale claims that IDL is absent or warning-only; assert `docs/cycles.md` does not call placeholder strings measured values; and assert that testnet/catalog/adoption remain marked pending while program ID/catalog/adoption evidence is absent. Keep patterns limited to these specific status claims.

- [ ] **Step 2: Run the consistency check before edits**

Expected: fail on the inaccurate claims that the chain job exercises a real sequencer and that measured cycle output is CI-generated/committed while the IDL is warning-only or absent.

- [ ] **Step 3: Update statements and criterion checkboxes from audit evidence**

Mark implementation as verified only after Tasks 1–9 commands passed. Keep testnet, catalog and adoption unchecked until owner action evidence is added. Link the owner checklist in the audit.

- [ ] **Step 4: Verify documentation consistency**

Run: `python3 tools/check_submission_docs.py`

Expected: success only when documentation status agrees with tracked IDL, CI checks and evidence files. Do not make external-owner criteria appear complete through prose alone.

## Owner actions after repository work

- [ ] Deploy to Logos testnet 0.4, test the deployed program and record program ID and registry PDA.
- [ ] Host real Geofabrik snapshots in Logos Storage and provide at least 15 covered countries / 25 checksum-verifiable entries with CIDs.
- [ ] Fork the catalog base, release both modules, and record the `logos-repo.json` URL.
- [ ] Obtain links/evidence for five independent public consumers, including at least three Basecamp UI apps, and Linux plus macOS use.
- [ ] Open the solution PR at `logos-co/lambda-prize` with the FURPS assessment and adoption evidence.
- [ ] Optionally collect evidence of >5 independent mirrors and genuine community vouches.

## Plan self-review

- Coverage: IDL generation/tracking/freshness, reported repc error, localnet and Storage-backed e2e, green CI gates, UI state, SDK consumer example, timestamp ordering, cycle results, and factual docs each have a task. External deployment, catalog, adoption and solution PR remain owner actions.
- TDD: each code task starts with a failing command or focused test where a practical test harness exists. The repc diagnosis step acknowledges the reported error but does not assume its root cause.
- Verification boundaries: Rust/IDL/docs tasks can be checked locally; repc, Basecamp package, real localnet/Storage CI and cycle instrumentation require CI/Nix/Docker infrastructure.
- No completion claim is made by this plan. No commands in the verification sections have been run in this planning phase.
