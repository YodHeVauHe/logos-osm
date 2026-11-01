# verify_coverage.py output: hosting node (node1), 2026-11-01 20:08 EAT

Command (run on the box that uploaded the data, through its own Storage session):

```
python3 tools/verify_coverage.py --logosctl <logosctl 0.3.2 AppRun> --storage-config <node1 session> --json-out coverage/verify-node1-2026-11-01.json
```

What this proves: the registry was read keylessly from the public testnet RPC, and every on-chain CID resolves in the hosting Storage node to bytes whose MD5 equals the on-chain checksum. Node1 already holds the blocks, so this is an integrity check of the hosted copies. It is **not** proof that another node can retrieve them over the network; see `docs/coverage-testnet-v0.3.md` → *Network availability*.

Raw output:

registry C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS via https://testnet.lez.logos.co: 25 entries on chain
fetching through Storage node 16Uiu2HAkuT4n3y8BmbXZXXavF6e6vp7qJ69eN58MVkxbqjCb3BXd

| Region | Storage CID | On-chain checksum | Fetched MD5 | Bytes | Seconds | Result |
|---|---|---|---|---|---|---|
| `china/henan` | `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP` | `54c5fbf47dc51588de2ffb0a0a94a771` | `54c5fbf47dc51588de2ffb0a0a94a771` | 50468082 | 0.7 | PASS |
| `china/shandong` | `zDvZRwzm2Pfy2E6VSL3coKQT6b3jd2Qs1NqB8nnXa2ZpMc1bZdix` | `936804b0029f858792fd074a480663e0` | `936804b0029f858792fd074a480663e0` | 71913523 | 0.8 | PASS |
| `china/jiangsu` | `zDvZRwzkyz2AjBB2yKn2rTr4k8BAqPvKRi9tWZCe3cUqPE7xEHkb` | `bd167b743906ef39ba008118559dcbfd` | `bd167b743906ef39ba008118559dcbfd` | 80205126 | 0.8 | PASS |
| `china/zhejiang` | `zDvZRwzm11a2mRKe6pEShG3Yq8UxXXhFyaXGKMwSYs4fJtbkb3Rf` | `bd4091f25fbb2db67113844e3d00bb68` | `bd4091f25fbb2db67113844e3d00bb68` | 91176749 | 0.8 | PASS |
| `china/sichuan` | `zDvZRwzmBQPWpmmLNVUYizuWY8CANm19gzNBVVRwRM1R23nBagHQ` | `33c8c07feaeb53d0a523380143e8a54f` | `33c8c07feaeb53d0a523380143e8a54f` | 107974694 | 0.8 | PASS |
| `china/guangdong` | `zDvZRwzm8zALCKjNP1yEbqXWFwtETbYhWNsDR4nAFfA85NqBTDdC` | `7ec207b0493f92d37549910fe87d5e72` | `7ec207b0493f92d37549910fe87d5e72` | 174433443 | 1.0 | PASS |
| `india/north-eastern-zone` | `zDvZRwzm9LEGekwHeE8hCfdnCLjC71nKZBCFJFXavdnBcG38s9jN` | `d6b00fd171ecde41f384047aa8913268` | `d6b00fd171ecde41f384047aa8913268` | 109874695 | 1.0 | PASS |
| `india/western-zone` | `zDvZRwzm8Pz4nAGzomqqSzuecuVgfgjcpiuZccL8r6zP8o39qbLD` | `f963ba20a61701711422c0e8ec843232` | `f963ba20a61701711422c0e8ec843232` | 221848192 | 1.4 | PASS |
| `india/northern-zone` | `zDvZRwzm9zdhc7VUCNkA6CjajjjMrTLmifioRABcQbRebZonn5fk` | `de5fd920288ee472cd0dce3ecc5531d1` | `de5fd920288ee472cd0dce3ecc5531d1` | 224446975 | 1.4 | PASS |
| `india/eastern-zone` | `zDvZRwzm5pYkevJoBNi9zVxo61bJFbN8HpGANZDDnRXSDinQMD3c` | `62eac0b8b77aee9a3a48946e01b4ae66` | `62eac0b8b77aee9a3a48946e01b4ae66` | 247523471 | 1.6 | PASS |
| `ethiopia` | `zDvZRwzm5LREspW9YAxns9wbRtgn3qZqB981vya6kZygozdw2uyH` | `05d69181c2ee748a37d6d5641982bd0c` | `05d69181c2ee748a37d6d5641982bd0c` | 140134692 | 1.1 | PASS |
| `kenya` | `zDvZRwzmAyrmSrDaqrVG5LVZ76edPDir5tV5suTSU6g2BpDsoVSw` | `e0407a073682244871fbb512c51f0827` | `e0407a073682244871fbb512c51f0827` | 351721422 | 2.0 | PASS |
| `egypt` | `zDvZRwzm4JnHgjkduxDD4EZ4gjoCPS7d6cJ87MjYXkaXznj6DFDY` | `a75319d9b4d57de7613c62c7b9e1d9f1` | `a75319d9b4d57de7613c62c7b9e1d9f1` | 178976151 | 1.2 | PASS |
| `morocco` | `zDvZRwzkzw66rhXUHg7SQLtYgocijZgP483Tv4QmVihZUt5Yycw1` | `b4a6bf3fe5bfa006f3e5d152f7292b6f` | `b4a6bf3fe5bfa006f3e5d152f7292b6f` | 244308777 | 1.4 | PASS |
| `pakistan` | `zDvZRwzmA8KcPfkuoaQiibjArwjnPrwUUwRjzXqS4EikDw2hTcqm` | `c6357e9b03e805cada430db19b9047e5` | `c6357e9b03e805cada430db19b9047e5` | 157641611 | 1.2 | PASS |
| `iran` | `zDvZRwzkzEzoK8M75GSvKSRhbk9J6PyJvXWDv2YjyuTVbytfmVVH` | `b2f9d62ec69cb9f342e3f3b940e1afe0` | `b2f9d62ec69cb9f342e3f3b940e1afe0` | 229960254 | 1.4 | PASS |
| `malaysia` | `zDvZRwzkyMiYmhNodDiDBCH6AUYDgTRzu8nDtfksfANndCZ5539w` | `da62ff73517c022dd8b8da32dd9153f5` | `da62ff73517c022dd8b8da32dd9153f5` | 252643071 | 1.5 | PASS |
| `south-korea` | `zDvZRwzm4xT6v3w2z971Cenvk8X1CDaX9KXS7ZuKDy1LuERZWgCU` | `12a7607c7a2dde1014406bbb45d1688c` | `12a7607c7a2dde1014406bbb45d1688c` | 290559413 | 1.5 | PASS |
| `thailand` | `zDvZRwzkwVVR423XAJGGAJ9QKo2gsrEsMXU2jBU9SsBtyoEypVdV` | `fbd9607f6d6ff4dca5a69d705183b82d` | `fbd9607f6d6ff4dca5a69d705183b82d` | 329616269 | 1.6 | PASS |
| `vietnam` | `zDvZRwzm2vs5TNhkBsd57frByUJXFDFDaqQmyvpCjVaPNFb5r9X2` | `0ddac22bd1f70f006ba107930da807ef` | `0ddac22bd1f70f006ba107930da807ef` | 332015990 | 1.5 | PASS |
| `bulgaria` | `zDvZRwzmAUzijGDu1Yb1TYXtZ7tJKfe1xuumn7U5oX1GaLm54FnU` | `9fd72538ac3851ef00b9de9c7f1e6ad4` | `9fd72538ac3851ef00b9de9c7f1e6ad4` | 176792601 | 1.1 | PASS |
| `hungary` | `zDvZRwzmDLgUVGoyHZhZ6DfAji1v1d2aYkKX2Sk9YXfN3GnytyVy` | `4df6e68bd5a90d97f9f1b28f74a83a4c` | `4df6e68bd5a90d97f9f1b28f74a83a4c` | 328171424 | 1.5 | PASS |
| `romania` | `zDvZRwzkyFeXPkX7WEEtgZrewy91TzNb3LZBSHyZvdH7kuWsgBoU` | `c78d3053dd01ddf3dfd3b6b13aa5d423` | `c78d3053dd01ddf3dfd3b6b13aa5d423` | 332504279 | 1.7 | PASS |
| `peru` | `zDvZRwzm53HDcgrv3DQopmvbYJNnvnvjuUbXpgg6asSdf8GyMdtA` | `95d7888520938bb3df5698617f25cdb3` | `95d7888520938bb3df5698617f25cdb3` | 257179453 | 1.7 | PASS |
| `colombia` | `zDvZRwzm2Y8igkEiXhdwxKMhF5hfb9wmQsZjrc9i4x2D9h932qYE` | `acadedb2675bdd32552439c98ce8372b` | `acadedb2675bdd32552439c98ce8372b` | 333642224 | 1.9 | PASS |

25/25 PASS across 17 countries
