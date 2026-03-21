# Remote WAN route-quality scoring

## Setup

- exact route only: `false`
- requested max route length: `3`
- candidate topology: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- local ingress: `127.0.0.1:19390`
- request path: `/perf-262144.bin`
- probe path: `/perf-262144.bin`
- runs: `6`

## Measurements

| run | status | connect ms | TTFB ms | total ms | body bytes | throughput Bps |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 200 | 0.83 | 2066.71 | 2067.89 | 262144 | 126768.59 |
| 2 | 200 | 0.70 | 2604.46 | 2605.85 | 262144 | 100598.35 |
| 3 | 200 | 0.76 | 1340.86 | 1342.02 | 262144 | 195334.85 |
| 4 | 200 | 0.95 | 1622.40 | 1623.83 | 262144 | 161435.34 |
| 5 | 200 | 0.73 | 1526.01 | 1527.21 | 262144 | 171649.43 |
| 6 | 200 | 0.89 | 1331.88 | 1333.10 | 262144 | 196642.40 |

Averages: total `1749.98 ms`, TTFB `1748.72 ms`, throughput `158738.16 Bps`.

## Candidate Scoring Evidence

- multi-candidate score events: `7`
- selections with quality metrics attached: `6`
- non-shortest selections observed: `7`
- feedback events received from exit: `30`

| selection | route len | selected route | score | reason | switched | score delta abs | score delta rel % | q conf sel/best | instability sel/best ppm | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | `initial_selection` | `false` | 0.0000 | 0.00 | 0.0000 / 0.0000 | - / - | - | - | - | - | - |
| 2 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7757 | `current_still_best` | `false` | 0.0000 | 0.00 | 0.1250 / 0.1250 | - / - | 1391 | 2661 | - | - | - |
| 3 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7674 | `current_still_best` | `false` | 0.0000 | 0.00 | 0.6250 / 0.6250 | 105847 / 105847 | 1319 | 2481 | 368 | 0 | 858192 |
| 4 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6911 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 97199 / 97199 | 1296 | 2471 | 336 | 0 | 903068 |
| 5 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7106 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 185763 / 185763 | 1003 | 2130 | 318 | 0 | 919973 |
| 6 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6845 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 461951 / 461951 | 773 | 1976 | 456 | 72496 | 932453 |
| 7 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7239 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 836849 / 836849 | 618 | 1839 | 362 | 24865 | 945442 |

Representative candidate snapshot:

| route len | candidate route | final score | q conf | warmup | instability ppm / factor | flap penalty | flap count | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |

Feedback events seen from exit:

| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `Udp://31.192.232.26:30000` | 557 | 166090 | 1212 | 1371 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 557 | 166090 | 1212 | 1371 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 557 | 166090 | 1212 | 1371 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 368 | 0 | 1168 | 1361 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 368 | 0 | 1168 | 1361 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 368 | 0 | 1168 | 1361 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 429 | 0 | 967 | 1056 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 429 | 0 | 967 | 1056 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 429 | 0 | 967 | 1056 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 276 | 0 | 919 | 1059 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 276 | 0 | 919 | 1059 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 276 | 0 | 919 | 1059 | 200 |

## Conclusion

The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.
