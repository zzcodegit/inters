# Remote WAN route-quality scoring

## Setup

- exact route only: `false`
- requested max route length: `3`
- candidate topology: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- local ingress: `127.0.0.1:19382`
- request path: `/perf-262144.bin`
- probe path: `/perf-262144.bin`
- runs: `4`

## Measurements

| run | status | connect ms | TTFB ms | total ms | body bytes | throughput Bps |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 200 | 0.41 | 2134.49 | 2135.11 | 262144 | 122777.82 |
| 2 | 200 | 0.43 | 2160.17 | 2160.90 | 262144 | 121312.29 |
| 3 | 200 | 0.40 | 2284.19 | 2284.83 | 262144 | 114732.63 |
| 4 | 200 | 0.62 | 1236.18 | 1237.05 | 262144 | 211911.02 |

Averages: total `1954.47 ms`, TTFB `1953.76 ms`, throughput `142683.44 Bps`.

## Candidate Scoring Evidence

- multi-candidate score events: `5`
- selections with quality metrics attached: `4`
- non-shortest selections observed: `5`
- feedback events received from exit: `21`

| selection | route len | selected route | score | reason | switched | score delta abs | score delta rel % | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | `initial_selection` | `false` | 0.0000 | 0.00 | - | - | - | - | - |
| 2 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7832 | `current_still_best` | `false` | 0.0000 | 0.00 | 285 | 1224 | - | - | - |
| 3 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7709 | `current_still_best` | `false` | 0.0000 | 0.00 | 552 | 1496 | 262 | 0 | 873345 |
| 4 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7711 | `current_still_best` | `false` | 0.0000 | 0.00 | 680 | 1694 | 269 | 0 | 878460 |
| 5 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7038 | `current_still_best` | `false` | 0.0000 | 0.00 | 1000 | 2036 | 357 | 779 | 928865 |

Representative candidate snapshot:

| route len | candidate route | final score | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | - | - | - | - | - |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | - | - | - | - | - |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | - | - | - | - | - |

Feedback events seen from exit:

| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |
| --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 262 | 0 | 924 | 1058 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 262 | 0 | 924 | 1058 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 262 | 0 | 924 | 1058 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 266 | 0 | 892 | 1058 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 266 | 0 | 892 | 1058 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 266 | 0 | 892 | 1058 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 274 | 0 | 934 | 1060 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 274 | 0 | 934 | 1060 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 274 | 0 | 934 | 1060 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 239 | 0 | 838 | 913 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 239 | 0 | 838 | 913 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 239 | 0 | 838 | 913 | 200 |

## Conclusion

The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.
