# Bulk host and local import on LEZ testnet v0.3 (2026-11-02)

Two real write paths of the `osm` CLI were run against the public LEZ testnet
v0.3 on 2026-11-02, under a hard budget of two transactions:

1. **Bulk host**: `osm host` with two unregistered regions, plus a third one
   opted out with `--exclude`. Both regions went into **one** `register_batch`
   transaction.
2. **Local import**: `osm import` of a PBF already on disk.

All times are EAT (UTC+3). Both registrations were then checked keylessly with
`tools/verify_coverage.py`.

## Setup

| Item | Value |
|---|---|
| LEZ | v0.3.1, RPC `https://testnet.lez.logos.co` |
| Program ID | `C31bCPXKa5uy1hzmegqMABWLbavoSJbY4Rhrc5yNXPrY` |
| Registry PDA | `C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS` |
| Signer / payer | `59VSwCJemEPWXBFGwQ8gzyZroNZ4AuQ81hJwpGUqMoEL` |
| Storage | logosctl 0.3.2 with storage_module 3.0.2, network `logos.test` (same pins and sha256 as `tools/coverage.sh`) |
| Regions file | [`coverage/regions-bulk-import-testnet-v0.3.json`](../coverage/regions-bulk-import-testnet-v0.3.json) |

Before the run, the registry held 25 entries, and none of them was
`greece`, `chile` or `belarus`.

## Budget guard

The runs were capped at two transactions, each with an explicit
`OSM_GAS_LIMIT`. `osm_registry_cli` preflights the fee reserve before it
submits:

```
reserve = gas_limit * next_base_fee_exec_ceiling + data_bytes * stor_ceiling
```

If the payer balance does not cover the reserve, it refuses to submit. The
budget did not allow a third transaction.

| Point | Payer balance |
|---|---|
| Before tx 1 (≈13:42) | 3,575,338 |
| After tx 1 | 1,910,442 |
| After tx 2 (final) | 249,698 |

The final 249,698 is below the reserve for any `register` call, so no further
testnet writes are possible from this account until it is topped up.

## Run 1: bulk host in one transaction

```sh
OSM_GAS_LIMIT=300000 osm --config osm-testnet.toml \
  host greece chile belarus --exclude belarus
```

```
hosted  chile   cid=zDvZRwzm4APx1ECXqsGe88HdcYTJDLGQHsncEQy61dkW7WcbRFFc version=261101 checksum=6bb93e6a0fe5e0aa4ac4db8b0a2b9c91
hosted  greece  cid=zDvZRwzm8vTvAtEuP2m9VHgQ5bp9mkFUkHBQWTZ2cqULiJMwd75F version=261101 checksum=63a580a3203235129ae274b2d26bd872

2 registered in one batch tx, 0 skipped
Submitted: 207214ce60a27d8e32a8bc5bf4c6654fc24c544fd1e4d41839da79765745bea0
fee preflight: payer balance 3575338, reserve ~2713230 (gas limit 300000)
fee paid: 1664896
```

- `--exclude` is applied before anything is downloaded, so `belarus` was
  never fetched, stored or registered by this command.
- Each PBF was downloaded from Geofabrik, checked against the published
  `.md5` sidecar, uploaded to Logos Storage, and then registered.
- Both entries landed in a **single** `register_batch` transaction. The
  sequencer's copy of the tx bytes contains `greece` and `chile` and does not
  contain `belarus`.
- That one transaction registered two regions for 1,664,896. The single-region
  import in run 2 cost 1,660,744, so batching roughly halves the per-region
  cost.

## Run 2: local import

```sh
# belarus-latest.osm.pbf was downloaded first; its MD5 equals Geofabrik's sidecar
OSM_GAS_LIMIT=210000 osm --config osm-testnet.toml \
  import belarus /abs/path/belarus-latest.osm.pbf
```

`osm import` checks the local file against Geofabrik's MD5
(`710f1f458db7bb745a44a0aa8207d8d6`), stores it, and registers it. The tx
contains `belarus` and that MD5.

**What actually happened (stated honestly):**

1. The first invocation's console session was torn down by the harness before
   its output was captured, so it looked aborted. Its transaction had in fact
   been submitted, and it landed in block 74323. This is tx 2 below.
2. A retry was started before the inclusion was visible. Its local preflight
   read the stale balance (1,910,442 ≥ reserve 1,901,205) and passed. The
   sequencer then **rejected** it with `InvalidParams: "Incorrect fee"`,
   because the payer could not fund it on top of the pending tx.
3. Nothing was charged for the rejected retry. The balances above account for
   exactly two fees: 3,575,338 − 1,664,896 − 1,660,744 = 249,698.

## Transactions

| # | What | Tx hash | Block | Time (EAT) | Gas limit | Fee paid |
|---|---|---|---|---|---|---|
| 1 | bulk `register_batch` of `greece` + `chile` (`belarus` excluded) | [`207214ce…bea0`](https://explorer.testnet.lez.logos.co/transaction/207214ce60a27d8e32a8bc5bf4c6654fc24c544fd1e4d41839da79765745bea0) | 74320 | 14:09:40 | 300,000 | 1,664,896 |
| 2 | `osm import belarus` | [`9314ed80…1966`](https://explorer.testnet.lez.logos.co/transaction/9314ed801a4d13ed889c19202bf7747f49c77653a9f08e39e3aefc859d001966) | 74323 | 14:14:23 | 210,000 | 1,660,744 |

Full hashes:

- `207214ce60a27d8e32a8bc5bf4c6654fc24c544fd1e4d41839da79765745bea0`
- `9314ed801a4d13ed889c19202bf7747f49c77653a9f08e39e3aefc859d001966`

Both hashes were also confirmed directly against the sequencer with
`getTransaction` on `https://testnet.lez.logos.co`. The block times come from
the block headers.

## Registry read-back

After both transactions the registry held 28 entries (25 before, 27 after
tx 1). The three new entries:

| Region | Version | Checksum (MD5) | Storage CID | Hosted | record_version |
|---|---|---|---|---|---|
| `greece` | 261101 | `63a580a3203235129ae274b2d26bd872` | `zDvZRwzm8vTvAtEuP2m9VHgQ5bp9mkFUkHBQWTZ2cqULiJMwd75F` | true | 1 |
| `chile` | 261101 | `6bb93e6a0fe5e0aa4ac4db8b0a2b9c91` | `zDvZRwzm4APx1ECXqsGe88HdcYTJDLGQHsncEQy61dkW7WcbRFFc` | true | 1 |
| `belarus` | 261101 | `710f1f458db7bb745a44a0aa8207d8d6` | `zDvZRwzm9NDGfdvauFx5bNCnVKkKqdYCiUCLW921Ua22mfEsFpXf` | true | 1 |

All three checksums equal the current Geofabrik `-latest.osm.pbf.md5`
sidecars, re-fetched after the run.

## Keyless verify

```sh
python3 tools/verify_coverage.py \
  --regions-file coverage/regions-bulk-import-testnet-v0.3.json \
  --json-out coverage/verify-bulk-import-2026-11-02.json
```

This was run at 14:19 EAT. It needs no wallet: it reads the registry PDA over
RPC, fetches each CID through logosctl, and compares the MD5 of the fetched
bytes with the on-chain checksum.

| Region | Bytes | On-chain = fetched MD5 | Result |
|---|---|---|---|
| `greece` | 342,588,747 | `63a580a3…d872` | PASS |
| `chile` | 350,897,984 | `6bb93e6a…9c91` | PASS |
| `belarus` | 351,381,112 | `710f1f45…07d8d6` | PASS |

**3/3 PASS.** Full output:
[`coverage/verify-bulk-import-2026-11-02.md`](../coverage/verify-bulk-import-2026-11-02.md)
and [`.json`](../coverage/verify-bulk-import-2026-11-02.json).

**Caveat:** this verify ran through the hosting node
(`16Uiu2HAkuT4n3y8BmbXZXXavF6e6vp7qJ69eN58MVkxbqjCb3BXd`). That explains the
fetch times of under 2 s. It proves that the registered CID resolves to bytes
matching the on-chain MD5. It does not prove retrieval by an independent
peer. Fresh-node retrieval is covered for the earlier set in
[`coverage-testnet-v0.3.md`](coverage-testnet-v0.3.md).

## Operational caveats found during the run

- **Do not override `TMPDIR`.** logosctl 0.3.2 finds its daemon through
  `/tmp/logos_core_service_*`. The first bulk attempt ran with a custom
  `TMPDIR`, and every Storage call failed with `NO_DAEMON`. Both regions were
  skipped (`0 registered`) and **no transaction was sent**. The rerun without
  the override is run 1 above.
- **Pass an absolute path to `osm import`.** The Storage daemon resolves the
  upload path against its own working directory, so a relative path fails at
  the upload step. `tools/run_localnet_registry_e2e.sh` uses absolute paths
  for this reason.
- **The preflight can be stale.** `osm_registry_cli` checks the reserve
  against the last confirmed balance. If a previous tx is still pending, the
  sequencer is the final judge, and it rejects with "Incorrect fee" without
  charging anything (see run 2).
