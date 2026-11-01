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
| Table generator | `tools/coverage_evidence_table.py` (fails if any on-chain CID or checksum differs from the verified local record) |

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
**22 regions across 14 countries** (from the registry read-back).

| Region | Country | Size | Geofabrik MD5 (= on-chain checksum) | Storage CID | Snapshot | Block | Tx |
|---|---|---|---|---|---|---|---|
| `bulgaria` | Bulgaria | 176.8 MB | `9fd72538ac3851ef00b9de9c7f1e6ad4` | `zDvZRwzmAUzijGDu1Yb1TYXtZ7tJKfe1xuumn7U5oX1GaLm54FnU` | 261031 | 73199 | [`340e05e8e346…`](https://explorer.testnet.lez.logos.co/transaction/340e05e8e3466b86095f82943f1c2382efe3cbf54e448e0e480c2a727f9b4522) |
| `china/guangdong` | China | 174.4 MB | `7ec207b0493f92d37549910fe87d5e72` | `zDvZRwzm8zALCKjNP1yEbqXWFwtETbYhWNsDR4nAFfA85NqBTDdC` | 261031 | 73104 | [`38881db15659…`](https://explorer.testnet.lez.logos.co/transaction/38881db15659c71c4d54a6cd69af80219b94ca55bfc02b5f0349e2eaafe6c4c1) |
| `china/henan` | China | 50.5 MB | `54c5fbf47dc51588de2ffb0a0a94a771` | `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP` | 261031 | 73088 | [`f3863dff2e22…`](https://explorer.testnet.lez.logos.co/transaction/f3863dff2e22d8208ea5fb3dab0a21fd9f5602b39033a9f089e313313fbe6f8e) |
| `china/jiangsu` | China | 80.2 MB | `bd167b743906ef39ba008118559dcbfd` | `zDvZRwzkyz2AjBB2yKn2rTr4k8BAqPvKRi9tWZCe3cUqPE7xEHkb` | 261031 | 73096 | [`a5d4f7b9fac9…`](https://explorer.testnet.lez.logos.co/transaction/a5d4f7b9fac9ec2a07ca45b49dd4dddcad4b01de837c05779c63dbea831b76a8) |
| `china/shandong` | China | 71.9 MB | `936804b0029f858792fd074a480663e0` | `zDvZRwzm2Pfy2E6VSL3coKQT6b3jd2Qs1NqB8nnXa2ZpMc1bZdix` | 261031 | 73094 | [`54521b5e4cc9…`](https://explorer.testnet.lez.logos.co/transaction/54521b5e4cc9aeb21ee29548c8b41f1b1c3b1ac3e59a46fbca15742df8450883) |
| `china/sichuan` | China | 108.0 MB | `33c8c07feaeb53d0a523380143e8a54f` | `zDvZRwzmBQPWpmmLNVUYizuWY8CANm19gzNBVVRwRM1R23nBagHQ` | 261031 | 73100 | [`f52ec64cc67d…`](https://explorer.testnet.lez.logos.co/transaction/f52ec64cc67dcef3a53fbdd63a5a5e2a2ec667863f30cd8802f6728f67acf24d) |
| `china/zhejiang` | China | 91.2 MB | `bd4091f25fbb2db67113844e3d00bb68` | `zDvZRwzm11a2mRKe6pEShG3Yq8UxXXhFyaXGKMwSYs4fJtbkb3Rf` | 261031 | 73099 | [`046adc936858…`](https://explorer.testnet.lez.logos.co/transaction/046adc93685858fc17789a3651eadc6152ca8480146671c4a0bae3e7ad275586) |
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
`logosctl` daemon, storage_module v3.0.2, with data under
`~/.logos_storage/data`. It is not replicated or pinned elsewhere. The registry
marks entries `hosted = true` because this node holds them, and they can be
fetched only while this box and its daemon are up. The daemon has run without a
restart during the run. There is no guarantee that it stays online after the
evidence run. If it goes away, the on-chain CIDs and checksums remain valid, but
the data would have to be re-seeded from Geofabrik (same snapshot, same MD5) to
be retrievable.
