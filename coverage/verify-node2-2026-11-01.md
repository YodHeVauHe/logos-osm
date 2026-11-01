# verify_coverage.py output: independent verifier node2, 2026-11-01 20:03 EAT

Fresh Storage node on the evidence box with its own HOME, session and empty data dir, bootstrapped only from logos.test. Command:

```
python3 tools/verify_coverage.py --storage-config <node2 session> --only china/henan --timeout 600 --evict
```

Raw output:

registry C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS via https://testnet.lez.logos.co: 25 entries on chain

| Region | Storage CID | On-chain checksum | Fetched MD5 | Bytes | Seconds | Result |
|---|---|---|---|---|---|---|
| `china/henan` | `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP` | `54c5fbf47dc51588de2ffb0a0a94a771` | `-` | - | 600.5 | FAIL (no storageDownloadDone within 600s) |

0/1 PASS across 0 countries
