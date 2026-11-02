# LP-0018 coverage evidence: LEZ testnet v0.3

Real Geofabrik extracts were downloaded, checked against Geofabrik's published
MD5, stored in a Logos Storage node, fetched back by CID and checked again, then
registered in the on-chain OSM registry on LEZ testnet v0.3.

| | |
|---|---|
| RPC | `https://testnet.lez.logos.co` (sequencer v0.3.1) |
| Program | `C31bCPXKa5uy1hzmegqMABWLbavoSJbY4Rhrc5yNXPrY` |
| Registry PDA | `C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS` |
| Fee payer | `Public/59VSwCJemEPWXBFGwQ8gzyZroNZ4AuQ81hJwpGUqMoEL` |
| Runner | `tools/host_coverage_regions.py` |
| Reproduce / verify | `tools/coverage.sh` (see [Reproduce](#reproduce)) |
| Table generator | `tools/coverage_evidence_table.py` (fails if any on-chain CID or checksum differs from the verified local record) |

## Reproduce

Everything is pinned in [`coverage/regions-testnet-v0.3.json`](../coverage/regions-testnet-v0.3.json):
LEZ `v0.3.1` (git tag in `examples/Cargo.toml`), `logosctl` `0.3.2` (tarball and
AppImage SHA-256 from the Logos docs), `storage_module` `3.0.2`, network
`logos.test`, plus the RPC, program id, registry PDA and the 25 regions.

```sh
tools/coverage.sh setup      # download + sha256-check logosctl 0.3.2, build osm and osm_registry_cli
tools/coverage.sh node       # start a Storage node on logos.test (built-in bootstrap SPRs)
tools/coverage.sh supervise  # watchdog: restarts daemon/storage_module only if they are down
tools/coverage.sh host       # needs a funded LEZ wallet; idempotent (see below)
tools/coverage.sh verify     # keyless; prints the pass/fail table
tools/coverage.sh status     # daemon, storage_module and supervisor health
```

The same steps are `make coverage-setup`, `coverage-node`, `coverage-host`,
`coverage-verify`, `coverage-verify-registry`, `coverage-supervise`,
`coverage-status`.

- **Idempotent host.** A region already in local state is skipped. A region not
  in local state but already registered on chain with `hosted = true` is
  skipped with `SKIP <region>: already registered on chain`, so a second
  machine does not register it again. Use `--gas-limit 500000` (the default
  passed by `coverage.sh`); see the fee root cause below.
- **Keyless verify.** `tools/verify_coverage.py` reads the registry with the
  sequencer's `getAccount` JSON-RPC and decodes the Borsh `Registry` itself. No
  wallet, password or key is used. For each region it downloads the on-chain CID
  through a Storage node (`downloadToUrl`, network mode), hashes the bytes and
  compares them with the on-chain checksum. `--registry-only` skips Storage.
  `--control-cid label=CID` also fetches a CID outside the registry, and
  `--evict` removes fetched data from a non-hosting verifier node.
- **Committed output.** [`coverage/verify-node1-2026-11-01.md`](../coverage/verify-node1-2026-11-01.md)
  (and `.json`): 25/25 PASS across 17 countries, run 2026-11-01 20:08 EAT
  through the hosting node. That proves the hosted bytes match the chain. It
  does not prove other nodes can fetch them; see the next section.

### Supervision

`tools/storage_supervisor.sh` checks `logosctl status` and
`storage_module isRunning` every 60 s and starts only what is down. A healthy
node is never restarted. The evidence box has no systemd (PID 1 is `tini`), so
it runs detached there:

```sh
LOGOSCTL_BIN=<AppRun> LOGOSCTL_CONFIG_DIR=<session> \
  setsid nohup tools/storage_supervisor.sh >> <session>/supervisor.log 2>&1 < /dev/null &
# check:
pgrep -af storage_supervisor.sh; tail <session>/supervisor.log
logosctl --config-dir <session> status
logosctl --config-dir <session> call storage_module isRunning
```

The detached watchdog survives the shell. It does not survive a reboot of the
box. On a host with systemd, use a unit instead:

```ini
# /etc/systemd/system/logos-osm-storage.service
[Unit]
Description=Logos Storage node for the OSM registry (logos.test)
After=network-online.target
Wants=network-online.target

[Service]
User=logos
Environment=HOME=/var/lib/logos-node
Environment=LOGOSCTL_BIN=/usr/local/bin/logosctl
Environment=LOGOSCTL_CONFIG_DIR=/var/lib/logos-node/.logosctl
ExecStart=/opt/logos-osm/tools/storage_supervisor.sh
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

## Pipeline per region

1. Fetch `<region>-latest.osm.pbf` and its `.md5` sidecar from Geofabrik. The
   redirect target gives the dated snapshot (`YYMMDD`). Stalled downloads are
   dropped and retried, and an MD5 mismatch is re-fetched once and then fails.
2. Run `osm prepare` to upload to the local Logos Storage node through `logosctl`
   and get the CID. Nothing is registered at this step.
3. Run `osm fetch-cid` to download the object back by CID. Its size and MD5 must
   equal the original's, or the region stops here.
4. Run `osm register-entry` to send a `Register` transaction signed by the fee
   payer.
5. Confirm with sequencer `getTransaction` (a block id) and a registry read-back
   whose `cid`, `checksum` and `hosted` match. This confirmation decides success,
   not the wallet's own poll (see below).

Local PBF and roundtrip copies are deleted after verification. The data stays in
the Storage node.

## Regions

<!-- regions:begin -->
**25 regions across 17 countries** (from the registry read-back).

| Region | Country | Size | Geofabrik MD5 (= on-chain checksum) | Storage CID | Snapshot | Block | Tx |
|---|---|---|---|---|---|---|---|
| `bulgaria` | Bulgaria | 176.8 MB | `9fd72538ac3851ef00b9de9c7f1e6ad4` | `zDvZRwzmAUzijGDu1Yb1TYXtZ7tJKfe1xuumn7U5oX1GaLm54FnU` | 261031 | 73199 | [`340e05e8e346…`](https://explorer.testnet.lez.logos.co/transaction/340e05e8e3466b86095f82943f1c2382efe3cbf54e448e0e480c2a727f9b4522) |
| `china/guangdong` | China | 174.4 MB | `7ec207b0493f92d37549910fe87d5e72` | `zDvZRwzm8zALCKjNP1yEbqXWFwtETbYhWNsDR4nAFfA85NqBTDdC` | 261031 | 73104 | [`38881db15659…`](https://explorer.testnet.lez.logos.co/transaction/38881db15659c71c4d54a6cd69af80219b94ca55bfc02b5f0349e2eaafe6c4c1) |
| `china/henan` | China | 50.5 MB | `54c5fbf47dc51588de2ffb0a0a94a771` | `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP` | 261031 | 73088 | [`f3863dff2e22…`](https://explorer.testnet.lez.logos.co/transaction/f3863dff2e22d8208ea5fb3dab0a21fd9f5602b39033a9f089e313313fbe6f8e) |
| `china/jiangsu` | China | 80.2 MB | `bd167b743906ef39ba008118559dcbfd` | `zDvZRwzkyz2AjBB2yKn2rTr4k8BAqPvKRi9tWZCe3cUqPE7xEHkb` | 261031 | 73096 | [`a5d4f7b9fac9…`](https://explorer.testnet.lez.logos.co/transaction/a5d4f7b9fac9ec2a07ca45b49dd4dddcad4b01de837c05779c63dbea831b76a8) |
| `china/shandong` | China | 71.9 MB | `936804b0029f858792fd074a480663e0` | `zDvZRwzm2Pfy2E6VSL3coKQT6b3jd2Qs1NqB8nnXa2ZpMc1bZdix` | 261031 | 73094 | [`54521b5e4cc9…`](https://explorer.testnet.lez.logos.co/transaction/54521b5e4cc9aeb21ee29548c8b41f1b1c3b1ac3e59a46fbca15742df8450883) |
| `china/sichuan` | China | 108.0 MB | `33c8c07feaeb53d0a523380143e8a54f` | `zDvZRwzmBQPWpmmLNVUYizuWY8CANm19gzNBVVRwRM1R23nBagHQ` | 261031 | 73100 | [`f52ec64cc67d…`](https://explorer.testnet.lez.logos.co/transaction/f52ec64cc67dcef3a53fbdd63a5a5e2a2ec667863f30cd8802f6728f67acf24d) |
| `china/zhejiang` | China | 91.2 MB | `bd4091f25fbb2db67113844e3d00bb68` | `zDvZRwzm11a2mRKe6pEShG3Yq8UxXXhFyaXGKMwSYs4fJtbkb3Rf` | 261031 | 73099 | [`046adc936858…`](https://explorer.testnet.lez.logos.co/transaction/046adc93685858fc17789a3651eadc6152ca8480146671c4a0bae3e7ad275586) |
| `colombia` | Colombia | 333.6 MB | `acadedb2675bdd32552439c98ce8372b` | `zDvZRwzm2Y8igkEiXhdwxKMhF5hfb9wmQsZjrc9i4x2D9h932qYE` | 261031 | 73265 | [`4b8f5df7844e…`](https://explorer.testnet.lez.logos.co/transaction/4b8f5df7844eb22123c09b99bf68c96dc6db00c952d6f3d985b3ef538ed7e6eb) |
| `egypt` | Egypt | 179.0 MB | `a75319d9b4d57de7613c62c7b9e1d9f1` | `zDvZRwzm4JnHgjkduxDD4EZ4gjoCPS7d6cJ87MjYXkaXznj6DFDY` | 261031 | 73135 | [`3e0d3c66c16d…`](https://explorer.testnet.lez.logos.co/transaction/3e0d3c66c16da74c8892cff651af43169f7d666d2e4fe0dc0b1bc5dcf3c092c8) |
| `ethiopia` | Ethiopia | 140.1 MB | `05d69181c2ee748a37d6d5641982bd0c` | `zDvZRwzm5LREspW9YAxns9wbRtgn3qZqB981vya6kZygozdw2uyH` | 261031 | 73125 | [`97b4ec6f09ba…`](https://explorer.testnet.lez.logos.co/transaction/97b4ec6f09ba5e24c1f0b413920a7b21bd7d58b6f102767a2214a3f0e4ec3f57) |
| `hungary` | Hungary | 328.2 MB | `4df6e68bd5a90d97f9f1b28f74a83a4c` | `zDvZRwzmDLgUVGoyHZhZ6DfAji1v1d2aYkKX2Sk9YXfN3GnytyVy` | 261031 | 73233 | [`0a28b2564929…`](https://explorer.testnet.lez.logos.co/transaction/0a28b25649298357d0fb6b95b28102c418cb63a5ac9c28b22cebcfd972ccecd7) |
| `india/eastern-zone` | India | 247.5 MB | `62eac0b8b77aee9a3a48946e01b4ae66` | `zDvZRwzm5pYkevJoBNi9zVxo61bJFbN8HpGANZDDnRXSDinQMD3c` | 261031 | 73123 | [`c5c6d32e6de8…`](https://explorer.testnet.lez.logos.co/transaction/c5c6d32e6de850a79c6867a51d8077e90b8245ab80c830af9e97007e1e802171) |
| `india/north-eastern-zone` | India | 109.9 MB | `d6b00fd171ecde41f384047aa8913268` | `zDvZRwzm9LEGekwHeE8hCfdnCLjC71nKZBCFJFXavdnBcG38s9jN` | 261031 | 73107 | [`fb5dc8bd9789…`](https://explorer.testnet.lez.logos.co/transaction/fb5dc8bd9789d283ef931384ec11894cb667baa2565453d6af5307f8986a78d9) |
| `india/northern-zone` | India | 224.4 MB | `de5fd920288ee472cd0dce3ecc5531d1` | `zDvZRwzm9zdhc7VUCNkA6CjajjjMrTLmifioRABcQbRebZonn5fk` | 261031 | 73118 | [`fee8be79a2d2…`](https://explorer.testnet.lez.logos.co/transaction/fee8be79a2d2f5e2e868c9e47ee78d3b54becf3555e3e65b7f9ecfed4ee09bc9) |
| `india/western-zone` | India | 221.8 MB | `f963ba20a61701711422c0e8ec843232` | `zDvZRwzm8Pz4nAGzomqqSzuecuVgfgjcpiuZccL8r6zP8o39qbLD` | 261031 | 73112 | [`57424f36b256…`](https://explorer.testnet.lez.logos.co/transaction/57424f36b256f2334f68ecf05e3207a41118b3aece5e2e6b5cf6207739adc47f) |
| `iran` | Iran | 230.0 MB | `b2f9d62ec69cb9f342e3f3b940e1afe0` | `zDvZRwzkzEzoK8M75GSvKSRhbk9J6PyJvXWDv2YjyuTVbytfmVVH` | 261031 | 73151 | [`af1f5ce6cfef…`](https://explorer.testnet.lez.logos.co/transaction/af1f5ce6cfefac4dcb87c89195c4decfb44304716b637e016277303ecab4aba4) |
| `kenya` | Kenya | 351.7 MB | `e0407a073682244871fbb512c51f0827` | `zDvZRwzmAyrmSrDaqrVG5LVZ76edPDir5tV5suTSU6g2BpDsoVSw` | 261031 | 73133 | [`16b8822db4d6…`](https://explorer.testnet.lez.logos.co/transaction/16b8822db4d6de387c3481e2a1b63d44ca88de8aa8552973e35dd6fd33a98a87) |
| `malaysia` | Malaysia | 252.6 MB | `da62ff73517c022dd8b8da32dd9153f5` | `zDvZRwzkyMiYmhNodDiDBCH6AUYDgTRzu8nDtfksfANndCZ5539w` | 261031 | 73173 | [`76f5535b86e5…`](https://explorer.testnet.lez.logos.co/transaction/76f5535b86e5cc385af7a5f888659d6e385218a1ec1a5f6e130dba0b7546e4f1) |
| `morocco` | Morocco | 244.3 MB | `b4a6bf3fe5bfa006f3e5d152f7292b6f` | `zDvZRwzkzw66rhXUHg7SQLtYgocijZgP483Tv4QmVihZUt5Yycw1` | 261031 | 73142 | [`a6e511d3fd10…`](https://explorer.testnet.lez.logos.co/transaction/a6e511d3fd102e9c2ff42a66b1e5c644b0a0f3ee1eb0cdc195e9f2381c6ef252) |
| `pakistan` | Pakistan | 157.6 MB | `c6357e9b03e805cada430db19b9047e5` | `zDvZRwzmA8KcPfkuoaQiibjArwjnPrwUUwRjzXqS4EikDw2hTcqm` | 261031 | 73146 | [`03de19923403…`](https://explorer.testnet.lez.logos.co/transaction/03de199234038a090c1c3b50a30f23415323060ca072bf16c4a9c1d87c3f5fab) |
| `peru` | Peru | 257.2 MB | `95d7888520938bb3df5698617f25cdb3` | `zDvZRwzm53HDcgrv3DQopmvbYJNnvnvjuUbXpgg6asSdf8GyMdtA` | 261031 | 73260 | [`c86f7e190bcb…`](https://explorer.testnet.lez.logos.co/transaction/c86f7e190bcb1c0d221efbd2fbaf3ed4af7a0be3a7c34e6295d178f341daf004) |
| `romania` | Romania | 332.5 MB | `c78d3053dd01ddf3dfd3b6b13aa5d423` | `zDvZRwzkyFeXPkX7WEEtgZrewy91TzNb3LZBSHyZvdH7kuWsgBoU` | 261031 | 73249 | [`3e7a46be96fd…`](https://explorer.testnet.lez.logos.co/transaction/3e7a46be96fd03bfff8b80492e8e3a58e7ba3c87675eff4e26fc4cae7034e2a3) |
| `south-korea` | South Korea | 290.6 MB | `12a7607c7a2dde1014406bbb45d1688c` | `zDvZRwzm4xT6v3w2z971Cenvk8X1CDaX9KXS7ZuKDy1LuERZWgCU` | 261031 | 73179 | [`7b2d951f626e…`](https://explorer.testnet.lez.logos.co/transaction/7b2d951f626edb48f353f39fa02c7dc48efbdc86a89c146bf9580278cb81bb0a) |
| `thailand` | Thailand | 329.6 MB | `fbd9607f6d6ff4dca5a69d705183b82d` | `zDvZRwzkwVVR423XAJGGAJ9QKo2gsrEsMXU2jBU9SsBtyoEypVdV` | 261031 | 73185 | [`b8df8ab5541f…`](https://explorer.testnet.lez.logos.co/transaction/b8df8ab5541f5cf427d4503ba3330372f23661dad55f8f8d97c561dfd0f83d45) |
| `vietnam` | Vietnam | 332.0 MB | `0ddac22bd1f70f006ba107930da807ef` | `zDvZRwzm2vs5TNhkBsd57frByUJXFDFDaqQmyvpCjVaPNFb5r9X2` | 261031 | 73216 | [`901f7a334646…`](https://explorer.testnet.lez.logos.co/transaction/901f7a334646481b87ea2fce5bc4b9d7416750595dfb27cb9882f3343c1c54a5) |
<!-- regions:end -->

Explorer note: when this doc was written, `explorer.testnet.lez.logos.co` showed
"Transaction not found" (and "No blocks found") even for transactions the
sequencer reports as included. The explorer indexer is lagging the chain. The
block column comes from the sequencer's `getTransaction`. You can re-check any row
like this:

```sh
curl -s https://testnet.lez.logos.co -H 'content-type: application/json' \
  -d '{"jsonrpc":"2.0","id":1,"method":"getTransaction","params":["<hash>"]}'
```

## Root cause of `InvalidParams "Incorrect fee"`

The v0.3.1 sequencer screens every transaction before admission
(`sequencer_core::fees::screen`). Any screening error comes back as the generic
RPC message "Incorrect fee". In this case the error was `PayerCannotFund`:

```
fee_reserve = gas_limit * next_base_fee_exec_ceiling + data_bytes * next_base_fee_stor_ceiling + tip
```

The wallet declares `gas_limit = 2_000_000` by default. With base fees of 8 and
ceilings of 9, the reserve was about 16M (18M at the ceiling). The payer held
15,383,554, so every new registration was refused. The cause was not nonces,
not a malformed fee, and not the registry account size: 20+ entries fit, and the
program is unchanged.

The charge for a `Register` is about 1.1–1.2M. That is roughly 140k execution
cycles plus storage bytes, far below the declared 2M gas.

Client fix (`examples/src/bin/osm_registry_cli.rs`):

- `--gas-limit` / `OSM_GAS_LIMIT` sets the declared gas. The coverage run used
  500,000, which keeps the reserve at about 4M while staying more than 3× above
  the measured cycles.
- A preflight reads `getFeeState` and the payer balance. It fails locally with an
  explicit "fee payer balance X is below the fee reserve Y" error instead of the
  opaque sequencer error. Unit tests cover the reserve formula and the default
  versus lowered gas limit.
- `--poll-blocks` / `OSM_POLL_BLOCKS` (default 40) sets how many blocks the
  wallet polls. The wallet config allowed only 5, so 11 earlier registrations were
  reported as failures ("Transaction not found in preconfigured amount of blocks")
  although every one landed. The runner now reconciles these through
  `getTransaction` and the read-back instead of resubmitting.
- Wallet config overrides (sequencer URL, gas, poll window) are now always
  applied. Before, they were applied only when `--rpc-url` was passed.

## Hosting caveat

All objects live in **one Logos Storage node on a single box**. It is a
`logosctl` 0.3.2 daemon with storage_module 3.0.2 and data under
`~/.logos_storage/data`, and since 2026-11-01 20:03 EAT it has been watched by
`tools/storage_supervisor.sh`. The data is not replicated anywhere else. The
registry marks entries `hosted = true` because this node holds them.

## Network availability

**Status (2026-11-02): an independent node on a GitHub runner downloaded
`china/henan` by its registry-resolved CID through Logos Storage, and the MD5
matched the on-chain checksum. See
[Retrieval through a reverse tunnel](#retrieval-through-a-reverse-tunnel-passed).
The hosting node is still behind NAT, so a node that does not open such a
tunnel still cannot reach it; the CIDs are not generally online yet.** The on-chain entries (CID, checksum,
source, snapshot) are valid regardless and can be re-seeded from Geofabrik with
the same MD5.

### How peers find the node

- `logosctl` 0.3.2 bundles `storage_module` 3.0.2 with a built-in `logos.test`
  preset: bootstrap SPRs, a DHT, and a mix relay pool. A fresh node with an
  empty data dir joins with no extra configuration (`tools/coverage.sh node`).
- Per the Logos Storage connectivity docs, with `nat = auto` the node tries
  UPnP / NAT-PMP / PCP port mapping, then a circuit relay, then hole punching.
  The docs warn that relay resources are limited and list "downloads time out
  when downloading from a different machine" as the symptom of a publisher
  behind NAT. The fix they give is a fixed `listen-port`, that TCP port open,
  and `nat = extip:<public IP>`.

### The evidence box is not reachable from outside (definite)

Checked 2026-11-01 20:25 EAT:

| Check | Result |
|---|---|
| Interface address | `172.30.0.2/24` (private), no public address on any interface |
| Public egress IPv4 | `104.28.195.115`, **AS13335 Cloudflare** (shared egress, not a host address) |
| Port mapping | daemon log repeatedly: `TCP port mapping failed ... plum: mapping 0 failed` |
| Advertised addresses (`storage_module debug`) | only `/p2p-circuit` relay addresses via `47.76.178.164:8070` and `128.140.55.128:8199`; no direct address |
| Relay health | repeated `libp2p relay ... Reservation timeout` heartbeats in the daemon log |
| TCP to egress IP on listen port 38371 | not reachable |

The box sits behind NAT on a Cloudflare egress; inbound connections can't reach
it, and other nodes can reach it only through relays. Nothing on this box can
change that.

### Independent retrieval attempts

All used `tools/verify_coverage.py` against fresh Storage nodes that have
their own HOME, session and empty data dir, and that bootstrap only from the
network.

| Verifier | Started (EAT) | Fetch | Result |
|---|---|---|---|
| node2 (box, peer `16Uiu2HAkwwAY…QCmsVV`) | 19:40 | `china/henan` (50.5 MB) | FAIL: connected to node1 over blockexc, streams closed every ~2 min, `storageDownloadDone` failure `Failed to stream file: Read stream failed` at 19:55:56, 0 bytes |
| node2, same node | 19:56 | public docs CID *Farewell to Westphalia* (control) | 0 bytes after 3+ min, stopped |
| node2, via `verify_coverage.py` | 20:03 | `china/henan` | FAIL: no `storageDownloadDone` within 600 s ([output](../coverage/verify-node2-2026-11-01.md)) |
| node3 (box, peer `16Uiu2HAmLmG8…qbo65bBP`, set up with `tools/coverage.sh node`) | 20:08 | control *Farewell to Westphalia* | FAIL: 0 bytes in 900 s |
| node3 | 20:23 | `kenya` + control | FAIL: no `storageDownloadDone` within 900 s, and the control got 0 bytes again (`coverage/verify-node3-2026-11-01.md`) |
| GitHub Actions run [41330147318](https://github.com/YodHeVauHe/logos-osm/actions/runs/41330147318) (fresh runner, relay-only) | 20:13 | control + `china/henan` + `kenya`, 1200 s each | FAIL, 0/2. Control: `Read stream failed`, 0 bytes. Henan: manifest fetch failed after 10 attempts dialing node2 (`…QCmsVV`, which had announced a partial copy) over `p2p-circuit`. The runner did reach node1 (`…Cb3BXd`) over blockexc, but its batch requests ended with `Read loop exited`. Kenya: no `storageDownloadDone` within 1200 s |
| GitHub Actions runs [41331443910](https://github.com/YodHeVauHe/logos-osm/actions/runs/41331443910), [41331566883](https://github.com/YodHeVauHe/logos-osm/actions/runs/41331566883) | 20:28 / 20:29 | `china/henan` + `kenya`, 600 s each | FAIL, 0/2 in both runs: no `storageDownloadDone` |

The box's own verifier nodes can't fetch even the public control file. So
from this box, a failed fetch says as much about the box's own NAT/relay path as
about the publisher. The GitHub runner is the independent check. It failed
too: content discovery works (the runner finds node1 as a provider), but
transfers between two relay-only peers break. The runner also could not fetch
the docs' public control CID, so the public testnet's relays are unreliable in
general. Log extracts are in [verify-gha-2026-11-01.md](../coverage/verify-gha-2026-11-01.md).

**Result (2026-11-01): over relays alone, the hosted data was not retrievable
from outside this box.**

### Retrieval through a reverse tunnel (passed)

The box allows only outbound HTTPS, so the hosting node dials out instead.
Workflow [`storage-retrieval-tunnel`](../.github/workflows/storage-retrieval-tunnel.yml)
starts a fresh Storage node on a GitHub runner with an empty data dir,
`listen-port 8070` and `nat = extip:127.0.0.1`, and exposes 8070 through a
Cloudflare quick tunnel (websocat WebSocket bridge). It publishes the wss URL,
peer ID and SPR as the `tunnel-info` artifact. On the box,
[`tools/storage_tunnel_peer.sh`](../tools/storage_tunnel_peer.sh) bridges
`127.0.0.1:8070` to that URL and re-inits the hosting node with the runner's SPR
as its only bootstrap node. The tunnel carries only the libp2p connection; the
blocks are served by storage_module over blockexchange.

(`storage_module connect` is not usable for this: through `logosctl` 0.3.2 it
returned `success=false` with no error and no `storageConnect` event for every
argument form tried, because its `[tstr]` address argument is not delivered.)

Run [41469069413](https://github.com/YodHeVauHe/logos-osm/actions/runs/41469069413),
2026-11-02 18:11–18:13 EAT:

| | |
|---|---|
| Downloader (runner, fresh) | `16Uiu2HAmAWEbJQshRYQCxmbav6FEVux4kVSYLyxKg9Xe4gvKtLvD` |
| Hosting node (box) | `16Uiu2HAm2cmq5yqh7YLDqbYVQ2b9B7NrnMPMwHd6AVmBwGswdjJX` |
| Registry | `C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS` via `https://testnet.lez.logos.co`, 28 entries read |
| Region / CID | `china/henan` / `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP` |
| On-chain checksum | `54c5fbf47dc51588de2ffb0a0a94a771` |
| Fetched MD5 | `54c5fbf47dc51588de2ffb0a0a94a771` |
| Bytes / time | 50,468,082 / 22.3 s |
| Result | **PASS** (1/1) |

The runner logged the inbound connection `127.0.0.1:8070 <- 127.0.0.1:59836`
(the tunnel) 30 s after start. The hosting node's daemon log shows
`Overriding network preset using custom bootstrap nodes` with the runner's peer,
then blockexchange streams with `16U*vKtLvD` (the runner) that closed at
18:12:51 EAT, two seconds after the runner's verify finished. Full output:
[verify-gha-tunnel-2026-11-02.md](../coverage/verify-gha-tunnel-2026-11-02.md).

Limits: one region was fetched, and the hosting node has to open the tunnel for
each run, so this proves retrieval by an independent node, not that the CIDs are
reachable by any node at any time. The hosting node here is a second box node
holding the same bytes (same manifest CID; it was uploaded as
`china__henan.osm.pbf` with block size 65536).

### What is needed to make it always online (needs the user)

A publicly reachable Storage node that holds the 25 CIDs (about 5 GB):

1. A small VPS with a public IPv4 (1–2 vCPU, 2 GB RAM, ≥ 20 GB disk;
   roughly US$5–10/month), or a home node with a port forward.
2. On it: `tools/coverage.sh setup` with `SKIP_BUILD=1`, then
   `STORAGE_LISTEN_PORT=8070 STORAGE_NAT=extip:<public IP> tools/coverage.sh node`,
   open TCP 8070, and install the systemd unit above.
3. Seed it with the exact bytes. Export each CID from the hosting node, which
   works locally (that is what the 25/25 verify did), copy the files to the
   VPS, and upload them there with `uploadUrl <file> 65536 true`. Then run
   `tools/coverage.sh verify` on a third machine; it checks the bytes against
   the on-chain MD5. `tools/coverage.sh host` alone will not re-seed, because it
   skips regions already on chain. Re-downloading from Geofabrik works only if
   the same dated snapshot (261031/261030) is still published.
4. Keep the `storage-retrieval` workflow as the public check. It runs on PRs that
   touch coverage files, and every 6 hours once merged.

Until then the honest statement is: **25 regions / 17 countries are registered
on chain with hosted copies whose bytes match the on-chain checksums
(25/25 PASS on the hosting node), and network retrieval by an independent node
is not yet demonstrated.**
