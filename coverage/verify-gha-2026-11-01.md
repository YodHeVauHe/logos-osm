# GitHub Actions independent retrieval, 2026-11-01

Fresh GitHub-hosted runners, pinned logosctl 0.3.2 / storage_module 3.0.2, empty data dir, bootstrap only. Extracted from the run logs (`gh run view <id> --log`). Times are UTC from the log; EAT is UTC+3.

## Run [41330147318](https://github.com/YodHeVauHe/logos-osm/actions/runs/41330147318)

```
2026-11-01T17:13:01.9251186Z registry C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS via https://testnet.lez.logos.co: 25 entries on chain
2026-11-01T17:13:01.9470236Z fetching through Storage node 16Uiu2HAmShYP9NLmLEZz6Gg71Fy1SxW2o61VwQYfTaAYYCxQ4RK1
2026-11-01T17:28:24.6886008Z control westphalia: FAIL 0 bytes Failed to stream file: Read stream failed
2026-11-01T17:48:45.0785145Z FAIL china/henan no storageDownloadDone within 1200s
2026-11-01T18:08:47.8205217Z FAIL kenya no storageDownloadDone within 1200s
2026-11-01T18:08:47.8210939Z | `china/henan` | `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP` | `54c5fbf47dc51588de2ffb0a0a94a771` | `-` | - | 1220.4 | FAIL (no storageDownloadDone within 1200s) |
2026-11-01T18:08:47.8212009Z | `kenya` | `zDvZRwzmAyrmSrDaqrVG5LVZ76edPDir5tV5suTSU6g2BpDsoVSw` | `e0407a073682244871fbb512c51f0827` | `-` | - | 1202.7 | FAIL (no storageDownloadDone within 1200s) |
2026-11-01T18:08:47.8213215Z control `westphalia` `zDvZRwzkzrrYB6sS1rRpRLt4gBhc1pWoyTSjkfszfmj1seaYYLCZ`: FAILED, 0 bytes in 922.7 s Failed to stream file: Read stream failed
2026-11-01T18:08:47.8213991Z 0/2 PASS across 0 countries
2026-11-01T18:08:47.8484475Z [2026-11-01 17:28:24.686] [out] [storage_module] WRN 2026-11-01 17:28:24.686+00:00 Block retries exhausted                    topics="storage activedownload" tid=2361 treeCid=zDz*kMhk7K exhaustedCount=1
2026-11-01T18:08:47.8485141Z [2026-11-01 17:28:24.686] [out] [storage_module] ERR 2026-11-01 17:28:24.686+00:00 Failed to STREAM.                          topics="libstorage libstoragedownload" tid=2361 error="Failed to stream file: Read stream failed"
2026-11-01T18:08:47.8494494Z [2026-11-01 17:34:11.541] [out] [storage_module] ERR 2026-11-01 17:34:11.540+00:00 Failed to MANIFEST.                        topics="libstorage libstoragedownload" tid=2361 error="Failed to fetch manifest: Failed to fetch manifest zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP after 10 attempts: Error fetching manifest from peer 16Uiu2HAkwwAYCKdYSiGhRfBEoRW8Zms4Gy3fKh3rNoSxH1QCmsVV
2026-11-01T18:08:47.8499542Z [2026-11-01 17:48:52.873] [out] [storage_module] WRN 2026-11-01 17:48:52.871+00:00 Batch request failed                       topics="storage blockexcengine" tid=2361 peer=16U*Cb3BXd error="Read loop exited"
2026-11-01T18:08:47.8500170Z [2026-11-01 17:48:52.873] [out] [storage_module] WRN 2026-11-01 17:48:52.871+00:00 Batch request failed                       topics="storage blockexcengine" tid=2361 peer=16U*Cb3BXd error="Read loop exited"
```

## Run [41331443910](https://github.com/YodHeVauHe/logos-osm/actions/runs/41331443910)

```
2026-11-01T17:28:18.6249893Z registry C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS via https://testnet.lez.logos.co: 25 entries on chain
2026-11-01T17:28:18.6561186Z fetching through Storage node 16Uiu2HAm5LkaJsn3UPpBgwd1fnzimWwxN8R1sLgRHuyMTn4VnwS1
2026-11-01T17:38:48.5455509Z FAIL china/henan no storageDownloadDone within 600s
2026-11-01T17:48:54.4864725Z FAIL kenya no storageDownloadDone within 600s
2026-11-01T17:48:54.4869080Z | `china/henan` | `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP` | `54c5fbf47dc51588de2ffb0a0a94a771` | `-` | - | 620.6 | FAIL (no storageDownloadDone within 600s) |
2026-11-01T17:48:54.4871531Z | `kenya` | `zDvZRwzmAyrmSrDaqrVG5LVZ76edPDir5tV5suTSU6g2BpDsoVSw` | `e0407a073682244871fbb512c51f0827` | `-` | - | 605.9 | FAIL (no storageDownloadDone within 600s) |
2026-11-01T17:48:54.4874191Z 0/2 PASS across 0 countries
2026-11-01T17:48:54.5267998Z [2026-11-01 17:34:13.167] [out] [storage_module] ERR 2026-11-01 17:34:13.166+00:00 Failed to MANIFEST.                        topics="libstorage libstoragedownload" tid=2498 error="Failed to fetch manifest: Failed to fetch manifest zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP after 10 attempts: Error fetching manifest from peer 16Uiu2HAkwwAYCKdYSiGhRfBEoRW8Zms4Gy3fKh3rNoSxH1QCmsVV
```

## Run [41331566883](https://github.com/YodHeVauHe/logos-osm/actions/runs/41331566883)

```
2026-11-01T17:29:44.7266712Z registry C5UwQUopzYvYREczjNJNf8fjmzrmf2QWK2f7Dd2YRgrS via https://testnet.lez.logos.co: 25 entries on chain
2026-11-01T17:29:44.7493955Z fetching through Storage node 16Uiu2HAmLbCHmeRTHjzSrVJAbh3o6fbzmsDAHeUuFBsKeG1YQqWS
2026-11-01T17:40:16.8202868Z FAIL china/henan no storageDownloadDone within 600s
2026-11-01T17:50:24.4653191Z FAIL kenya no storageDownloadDone within 600s
2026-11-01T17:50:24.4658988Z | `china/henan` | `zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP` | `54c5fbf47dc51588de2ffb0a0a94a771` | `-` | - | 620.5 | FAIL (no storageDownloadDone within 600s) |
2026-11-01T17:50:24.4660411Z | `kenya` | `zDvZRwzmAyrmSrDaqrVG5LVZ76edPDir5tV5suTSU6g2BpDsoVSw` | `e0407a073682244871fbb512c51f0827` | `-` | - | 607.6 | FAIL (no storageDownloadDone within 600s) |
2026-11-01T17:50:24.4662745Z 0/2 PASS across 0 countries
2026-11-01T17:50:24.4962977Z [2026-11-01 17:35:40.071] [out] [storage_module] ERR 2026-11-01 17:35:40.070+00:00 Failed to MANIFEST.                        topics="libstorage libstoragedownload" tid=2280 error="Failed to fetch manifest: Failed to fetch manifest zDvZRwzm5FYsh7BU1kK1TzjuZU92FyLFvqMjGUm5zm9pzWMLMUSP after 10 attempts: Error fetching manifest from peer 16Uiu2HAkwwAYCKdYSiGhRfBEoRW8Zms4Gy3fKh3rNoSxH1QCmsVV
2026-11-01T17:50:24.4972707Z [2026-11-01 17:40:32.840] [out] [storage_module] WRN 2026-11-01 17:40:32.839+00:00 Batch request failed                       topics="storage blockexcengine" tid=2280 peer=16U*Cb3BXd error="Read loop exited"
2026-11-01T17:50:24.4973601Z [2026-11-01 17:40:32.840] [out] [storage_module] WRN 2026-11-01 17:40:32.839+00:00 Batch request failed                       topics="storage blockexcengine" tid=2280 peer=16U*Cb3BXd error="Read loop exited"
```

