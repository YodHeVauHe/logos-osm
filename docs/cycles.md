# Cycle count methodology

The guest measures instruction execution with `risc0_zkvm::guest::env::cycle_count()` immediately before and after registry work. It logs `register_region` with one entry and `register_batch` with the submitted entry count. The real-backend e2e test submits a two-entry batch.

## Measured localnet sample

The following values were read from `lgs localnet logs --tail 500` after a
passing `localnet_registry_host_query` run against the real sequencer. The
guest source is commit `29779bd052031ff60a56c1e356a9e038f4b3b1bb`, producing
Program ID
`8fd44f40975a635ba9c4a2fb8f91118c0cfcc060831e1a964d8d1583abadbad4`.
The SDK/e2e harness at the first measured run was commit `1d60991209ec3343c8debd799b13a38cf56408d0`.

| Operation | Input | Guest cycles |
|---|---:|---:|
| `register_region` | 1 entry (Germany) | 2,145 |
| `register_batch` | 2 entries (Germany and Kenya) | 22,785 |

A subsequent passing run against the same Program ID also emitted 10,194
cycles for its one-entry registration and 26,483 for its two-entry batch.
These are separate observed samples: registry payload fields differed, so
cycle counts are workload-specific rather than a fixed operation constant.

The measured environment used `cargo-risczero 3.0.6`, RISC Zero `r0vm 3.0.6`,
guest `risc0-zkvm 3.0.5`, Rust `1.97.0`, and LEZ commit
`27360cb7d6ccb2bfbcca7d171bab8a3938490264`. `cycle_count()` is read in the
guest immediately around registry work; the sequencer log's `R0VM[...] CU ...`
line is the source of the values above. The chain workflow requires nonzero
records for one-entry `register_region` and two-entry `register_batch`, then
uploads the extracted lines as the `cycles` artifact.

To reproduce locally, build and deploy the guest using the pinned RISC Zero toolchain, start the configured standalone LEZ localnet, create and fund a signing wallet account, and run `localnet_registry_host_query` with `LGS_LOCALNET=1` and the registry CLI, IDL, program, wallet-home, and signer variables. This path uses in-memory object storage and does not require Logos Storage. Capture the sequencer output:

```sh
lgs localnet logs --tail 500 > localnet.log
grep -E 'CU register_region entries=1 cycles=[1-9][0-9]*' localnet.log
grep -E 'CU register_batch entries=2 cycles=[1-9][0-9]*' localnet.log
```

The LEZ repository also provides [`cycle_bench`](https://github.com/logos-blockchain/logos-execution-zone/tree/dev/tools/cycle_bench) for operation-level benchmarking. Report only observed outputs and declare the batch size.
