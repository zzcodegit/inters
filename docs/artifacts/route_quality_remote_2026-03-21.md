# Remote WAN route-quality scoring

## Setup

- exact route only: `false`
- requested max route length: `3`
- candidate topology: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- local ingress: `127.0.0.1:19380`
- request path: `/perf-262144.bin`
- probe path: `/perf-262144.bin`
- runs: `6`

## Measurements

| run | status | connect ms | TTFB ms | total ms | body bytes | throughput Bps |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 200 | 0.90 | 2453.62 | 2454.94 | 262144 | 106782.13 |
| 2 | 200 | 1.20 | 2244.37 | 2246.46 | 262144 | 116691.79 |
| 3 | 200 | 0.83 | 1314.77 | 1316.06 | 262144 | 199187.78 |
| 4 | 200 | 0.92 | 1165.03 | 1166.40 | 262144 | 224745.90 |
| 5 | 200 | 0.97 | 1194.76 | 1196.31 | 262144 | 219127.46 |
| 6 | 200 | 0.96 | 1165.23 | 1166.64 | 262144 | 224700.74 |

Averages: total `1591.14 ms`, TTFB `1589.63 ms`, throughput `181872.63 Bps`.

## Candidate Scoring Evidence

- multi-candidate score events: `7`
- selections with quality metrics attached: `6`
- non-shortest selections observed: `7`
- feedback events received from exit: `27`

| selection | route len | selected route | score | reason | switched | score delta abs | score delta rel % | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | `initial_selection` | `false` | 0.0000 | 0.00 | - | - | - | - | - |
| 2 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6754 | `current_still_best` | `false` | 0.0000 | 0.00 | 4190 | 7336 | 1018 | 444444 | 976470 |
| 3 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6598 | `current_still_best` | `false` | 0.0000 | 0.00 | 3362 | 5869 | 1018 | 444444 | 976470 |
| 4 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6378 | `current_still_best` | `false` | 0.0000 | 0.00 | 2314 | 4029 | 393 | 52287 | 851858 |
| 5 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6943 | `current_still_best` | `false` | 0.0000 | 0.00 | 1713 | 3213 | 307 | 6150 | 867932 |
| 6 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7204 | `current_still_best` | `false` | 0.0000 | 0.00 | 1291 | 2596 | 307 | 6150 | 867932 |
| 7 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7462 | `current_still_best` | `false` | 0.0000 | 0.00 | 996 | 2170 | 294 | 2109 | 831124 |

Representative candidate snapshot:

| route len | candidate route | final score | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | - | - | - | - | - |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | - | - | - | - | - |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | - | - | - | - | - |

Feedback events seen from exit:

| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `Udp://31.192.232.26:30000` | 1657 | 658621 | 3146 | 3327 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 1657 | 658621 | 3146 | 3327 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 1657 | 658621 | 3146 | 3327 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 1018 | 444444 | 2656 | 2720 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 1018 | 444444 | 2656 | 2720 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 1018 | 444444 | 2656 | 2720 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 1499 | 666667 | 2365 | 2567 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 1499 | 666667 | 2365 | 2567 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 1499 | 666667 | 2365 | 2567 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 369 | 0 | 1036 | 1209 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 369 | 0 | 1036 | 1209 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 369 | 0 | 1036 | 1209 | 200 |

## Conclusion

The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.
