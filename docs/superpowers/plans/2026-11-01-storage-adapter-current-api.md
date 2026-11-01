# Plan: current Logos Storage adapter

1. Add unit tests first for `logosctl call` argv, nested result parsing, completion event/session matching, failed events, and timeout.
2. Implement `LogosctlStorage` with daemon health/start handling, watch-before-call sequencing, event waits, destination validation, and existing retry policy. Preserve `StorageBackend` and `InMemoryStorage` tests.
3. Replace CLI configuration/docs references to the legacy binary and add an env-gated ignored live roundtrip test.
4. Run targeted tests, formatting, clippy, requested SDK/CLI tests, and documentation checker. Run the real adapter against the supplied node and checksum the requested PBF.
5. Update status claims only from produced evidence and commit docs, implementation, and evidence in small commits. Do not commit generated files, datasets, or session data.
