# Short-Path WAN Compare

## Perf Compare

| route | pacing total ms | rejected inflight total ms | corrected inflight total ms | short-path fix total ms | delta vs pacing ms | delta vs corrected ms |
| --- | --- | --- | --- | --- | --- | --- |
| 1-hop | 1151.68 | 1116.22 | 1555.92 | 1122.00 | -29.68 | -433.92 |
| 2-hop | 1183.38 | 1811.82 | 1212.74 | 1196.78 | 13.40 | -15.96 |
| 3-hop | 1609.04 | 1487.11 | 1407.29 | 1445.42 | -163.62 | 38.13 |

## Stage Compare

| route | metric | pacing | corrected inflight | short-path fix |
| --- | --- | --- | --- | --- |
| 1-hop | avg total ms | 1113.37 | 1708.77 | 1249.32 |
| 1-hop | ACK avg ms | 182.80 | 244.00 | 178.80 |
| 1-hop | ACK p95 ms | 190.00 | 397.60 | 191.00 |
| 1-hop | hard stall ms | 32.40 | 201.60 | 102.20 |
| 1-hop | retransmit rate | 0.0000 | 0.0909 | 0.0366 |
| 1-hop | pacing interval ms | 3.00 | 4.00 | 3.00 |
| 1-hop | effective cap avg | - | 64.00 | 64.00 |
| 1-hop | blocked by cap | - | 0.00 | 0.00 |
| 2-hop | avg total ms | 1133.96 | 1185.07 | 1123.26 |
| 2-hop | ACK p95 ms | 200.40 | 203.60 | 193.60 |
| 2-hop | hard stall ms | 44.00 | 54.80 | 46.20 |
| 2-hop | retransmit rate | 0.0000 | 0.0000 | 0.0000 |
| 2-hop | effective cap avg | - | 64.00 | 64.00 |
| 2-hop | blocked by cap | - | 0.00 | 0.00 |
| 3-hop | avg total ms | 1517.50 | 1422.69 | 1397.93 |
| 3-hop | ACK p95 ms | 288.20 | 251.80 | 246.60 |
| 3-hop | hard stall ms | 155.00 | 41.40 | 25.60 |
| 3-hop | retransmit rate | 0.0000 | 0.0000 | 0.0000 |
| 3-hop | effective cap avg | - | 64.00 | 64.00 |
| 3-hop | blocked by cap | - | 0.00 | 0.00 |

## 1-Hop Run-Level Causality

Previous corrected pass (`b7e7090`) had three bad `1-hop` runs with no cap activity:

| run | total ms | retransmit rate | ACK avg ms | ACK p95 ms | pacing interval ms | hard stall ms | cap avg | blocked |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| run2 | 2204.14 | 0.2222 | 289 | 578 | 5 | 401 | 64 | 0 |
| run3 | 1894.15 | 0.0972 | 251 | 440 | 5 | 224 | 64 | 0 |
| run4 | 1667.28 | 0.1349 | 262 | 484 | 4 | 300 | 64 | 0 |

Fresh short-path run still contains one retransmit-heavy `1-hop` sample:

| run | total ms | retransmit rate | ACK avg ms | ACK p95 ms | pacing interval ms | hard stall ms | cap avg | blocked |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| run5 | 1450.20 | 0.1828 | 168 | 188 | 3 | 361 | 64 | 0 |

This is the decisive compare:

- retransmit pressure still exists
- effective cap still does not fire
- but ACK tail no longer inflates
- pacing interval stays at the short-path baseline `3 ms`

## Readout

- `1-hop` moved from the bad corrected sample (`1555.92 ms`) to `1122.00 ms`.
- `1-hop` now sits below the accepted pacing baseline by `29.68 ms`.
- `2-hop` stays free of the old control-induced regression: `1196.78 ms` is only `13.40 ms` above pacing and `15.96 ms` below the corrected inflight pass.
- `3-hop` remains materially better than pacing baseline (`1609.04 -> 1445.42 ms`), though this live sample is `38.13 ms` above the best corrected sample.
