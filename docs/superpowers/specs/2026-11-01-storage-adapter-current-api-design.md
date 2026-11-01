# Current Logos Storage adapter design

## Goal

Replace the legacy `LogoscoreStorage` calls with a `LogosctlStorage` adapter for the current `storage_module` methods while preserving `StorageBackend` and the in-memory implementation.

## Design

The adapter receives a `logosctl` executable and session `--config-dir`. Every operation calls the persistent daemon through `logosctl call`; it never owns or stops the daemon. Before storage work it checks `isRunning` and requests `start` if needed. Upload and download start a `watch storage_module` process before issuing the operation, then correlate completion JSONL events by session ID. Upload returns the event CID and local file size; download checks completion and destination existence. Event failures, command errors, and bounded waits are surfaced as storage errors and remain eligible for existing retry backoff.

Argument construction and JSON result/event parsing are pure helpers. The configured session directory is provided by the caller; the adapter does not embed daemon secrets or invent module initialization configuration. CLI configuration becomes `logosctl_bin` and `storage_config_dir`. Existing FFI configuration keys are retained only where needed for source compatibility and mapped to the new values.

## Verification

Unit tests cover argv, command result parsing, event correlation, failure events, and timeout. A separately gated live test uses the real adapter and configured daemon. Evidence records a single-node local storage roundtrip only; it does not establish a multi-node/network transfer.
