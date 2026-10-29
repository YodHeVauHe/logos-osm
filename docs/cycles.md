# Cycle count methodology

The guest measures instruction execution with `risc0_zkvm::guest::env::cycle_count()` immediately before and after registry work. It logs `register_region` with one entry and `register_batch` with the submitted entry count. The real-backend e2e test submits a two-entry batch.

No measured numeric values are recorded in this repository yet. The chain workflow captures sequencer logs after the e2e test, requires nonzero cycle records for `register_region` (one entry) and `register_batch` (two entries), and uploads the extracted lines as the `cycles` artifact. The workflow does not write measurements into this document. Numeric values should be committed here only from a successful artifact, along with the RISC Zero toolchain and sequencer versions.

To reproduce locally, build and deploy the guest using the pinned RISC Zero toolchain, start the configured standalone LEZ localnet, run the `localnet_host_query_download` e2e test with `LGS_LOCALNET=1` and the required registry and Logos Storage variables, then capture the sequencer output:

```sh
lgs localnet logs --tail 500 > localnet.log
grep -E 'CU register_region entries=1 cycles=[1-9][0-9]*' localnet.log
grep -E 'CU register_batch entries=2 cycles=[1-9][0-9]*' localnet.log
```

The LEZ repository also provides [`cycle_bench`](https://github.com/logos-blockchain/logos-execution-zone/tree/dev/tools/cycle_bench) for operation-level benchmarking. Report only observed outputs and declare the batch size.
