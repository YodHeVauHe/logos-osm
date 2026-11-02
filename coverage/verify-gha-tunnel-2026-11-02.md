# Retrieval by an independent node through a reverse tunnel (2026-11-02)

GitHub Actions run https://github.com/YodHeVauHe/logos-osm/actions/runs/41469069413, artifact `storage-retrieval-tunnel/verify.md`:

```
registry C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS via https://testnet.lez.logos.co: 28 entries on chain
fetching through Storage node 16Uiu2HAmAWEbJQshRYQCxmbav6FEVux4kVSYLyxKg9Xe4gvKtLvD

| Region | Storage CID | On-chain checksum | Fetched MD5 | Bytes | Seconds | Result |
|---|---|---|---|---|---|---|
| `china/henan` | `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP` | `54c5fbf47dc51588de2ffb0a0a94a771` | `54c5fbf47dc51588de2ffb0a0a94a771` | 50468082 | 22.3 | PASS |

1/1 PASS across 1 countries
```

Runner wait step:

```
inbound libp2p connection through the tunnel after 30 s
Recv-Q Send-Q Local Address:Port Peer Address:Port 
0      0          127.0.0.1:8070    127.0.0.1:59836
```

Hosting node daemon log (box, EAT):

```
[2026-11-02 18:12:24.362] [info] [logos] [storage_module] StorageModuleImpl::init called
[2026-11-02 18:12:24.375] [out] [storage_module] WRN 2026-11-02 18:12:24.371+03:00 Overriding network preset using custom bootstrap nodes topics="libstorage libstoragelifecycle" tid=3029366 nodes="@[(peerId: 16Uiu2HAmAWEbJQshRYQCxmbav6FEVux4kVSYLyxKg9Xe4gvKtLvD, ...)]"
[2026-11-02 18:12:24.426] [info] [logos] [storage_module] StorageModuleImpl::start called
[2026-11-02 18:12:24.429] [out] [storage_module] NTC 2026-11-02 18:12:24.427+03:00 Started Storage node                       topics="storage node" tid=3029366 id=16U*swdjJX addrs=[]
[2026-11-02 18:12:51.293] [out] [storage_module] WRN 2026-11-02 18:12:51.291+03:00 Detaching read loop                        topics="storage blockexcnetworkpeer" tid=3029366 peer=16U*vKtLvD connId=6ae8a85f1d37533dbe7ab7c9
[2026-11-02 18:12:51.293] [out] [storage_module] WRN 2026-11-02 18:12:51.291+03:00 Detaching read loop                        topics="storage blockexcnetworkpeer" tid=3029366 peer=16U*vKtLvD connId=6ae8a85f1d37533dbe7ab7ca
```
