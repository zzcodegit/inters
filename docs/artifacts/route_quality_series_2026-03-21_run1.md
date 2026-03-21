# Remote WAN route-quality scoring

## Setup

- exact route only: `false`
- requested max route length: `3`
- candidate topology: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- local ingress: `127.0.0.1:19381`
- request path: `/perf-262144.bin`
- probe path: `/perf-262144.bin`
- runs: `4`

## Measurements

| run | status | connect ms | TTFB ms | total ms | body bytes | throughput Bps |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 200 | 0.39 | 2111.66 | 2112.25 | 262144 | 124106.37 |
| 2 | 200 | 0.41 | 5553.15 | 5553.93 | 262144 | 47199.75 |
| 3 | 200 | 4.67 | 2062.20 | 2067.12 | 262144 | 126815.84 |
| 4 | 200 | 0.42 | 1247.65 | 1248.31 | 262144 | 209999.96 |

Averages: total `2745.40 ms`, TTFB `2743.67 ms`, throughput `127030.48 Bps`.

## Candidate Scoring Evidence

- multi-candidate score events: `5`
- selections with quality metrics attached: `4`
- non-shortest selections observed: `5`
- feedback events received from exit: `21`

| selection | route len | selected route | score | reason | switched | score delta abs | score delta rel % | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | `initial_selection` | `false` | 0.0000 | 0.00 | - | - | - | - | - |
| 2 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7811 | `current_still_best` | `false` | 0.0000 | 0.00 | 288 | 1982 | - | - | - |
| 3 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7094 | `current_still_best` | `false` | 0.0000 | 0.00 | 515 | 2020 | 952 | 441379 | 923925 |
| 4 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6473 | `current_still_best` | `false` | 0.0000 | 0.00 | 1125 | 3079 | 1666 | 708926 | 956008 |
| 5 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6164 | `current_still_best` | `false` | 0.0000 | 0.00 | 1547 | 3006 | 753 | 243161 | 927725 |

Representative candidate snapshot:

| route len | candidate route | final score | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | - | - | - | - | - |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | - | - | - | - | - |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | - | - | - | - | - |

Feedback events seen from exit:

| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |
| --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 952 | 441379 | 1676 | 1814 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 952 | 441379 | 1676 | 1814 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 952 | 441379 | 1676 | 1814 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 238 | 0 | 821 | 906 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 238 | 0 | 821 | 906 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 238 | 0 | 821 | 906 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 327 | 0 | 1072 | 1210 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 327 | 0 | 1072 | 1210 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 327 | 0 | 1072 | 1210 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 1588 | 827586 | 2238 | 2419 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 1588 | 827586 | 2238 | 2419 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 1588 | 827586 | 2238 | 2419 | 200 |

## Conclusion

The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.
