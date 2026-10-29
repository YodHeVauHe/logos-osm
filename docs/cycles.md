# Cycle count methodology

The LEZ v0.3.1 guest reads `risc0_zkvm::guest::env::cycle_count()` around registry planning and logs:

- `CU register_region entries=1 cycles=N`
- `CU register_batch entries=N cycles=N`

## LEZ v0.3.1 localnet sample

Captured on 2026-10-29 with the passing `tools/run_localnet_registry_e2e.sh` run against the standalone LEZ v0.3.1 sequencer in RISC Zero dev mode. The guest logged each invocation twice in the sequencer output; both copies agreed.

| Operation | Input | Guest cycles (v0.3.1) |
|---|---:|---:|
| `register_region` | 1 entry (Germany) | 3,801 |
| `register_batch` | 2 entries (Germany and Kenya) | 4,877 |

## Historical pre-port measurements

The values below came from the previous SPEL/LEZ revision `27360cb7d6ccb2bfbcca7d171bab8a3938490264` and are retained as historical data only. They are not measurements of the v0.3.1 guest and must not be used as its performance claims.

| Operation | Input | Guest cycles (pre-port) |
|---|---:|---:|
| `register_region` | 1 entry (Germany) | 2,145 |
| `register_batch` | 2 entries (Germany and Kenya) | 22,785 |
| `register_region` | 1 entry (separate observed sample) | 10,194 |
| `register_batch` | 2 entries (separate observed sample) | 26,483 |

The pre-port guest ProgramId was `8fd44f40975a635ba9c4a2fb8f91118c0cfcc060831e1a964d8d1583abadbad4`. The current guest image ID is recorded in README; the two identifiers belong to different guest builds.

## Reproduction

Build with `make build`, deploy the produced ProgramBinary using the LEZ v0.3.1 wallet's `program-loader deploy`, then run `tools/run_localnet_registry_e2e.sh` with the LEZ wallet and sequencer binaries configured. The script enables `RISC0_DEV_MODE=1` and prints the sequencer's cycle output after a successful run.
