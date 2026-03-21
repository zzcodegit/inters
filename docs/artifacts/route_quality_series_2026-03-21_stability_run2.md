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
| 1 | 200 | 0.55 | 1930.89 | 1931.66 | 262144 | 135709.44 |
| 2 | 200 | 0.49 | 1962.19 | 1963.01 | 262144 | 133541.63 |
| 3 | 200 | 0.42 | 2108.82 | 2109.45 | 262144 | 124271.09 |
| 4 | 200 | 0.41 | 1243.00 | 1243.64 | 262144 | 210787.25 |

Averages: total `1811.94 ms`, TTFB `1811.23 ms`, throughput `151077.35 Bps`.

## Candidate Scoring Evidence

- multi-candidate score events: `5`
- selections with quality metrics attached: `4`
- non-shortest selections observed: `5`
- feedback events received from exit: `21`

| selection | route len | selected route | score | reason | switched | score delta abs | score delta rel % | q conf sel/best | instability sel/best ppm | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | `initial_selection` | `false` | 0.0000 | 0.00 | 0.0000 / 0.0000 | - / - | - | - | - | - | - |
| 2 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7779 | `current_still_best` | `false` | 0.0000 | 0.00 | 0.1250 / 0.1250 | - / - | 255 | 1301 | - | - | - |
| 3 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7680 | `current_still_best` | `false` | 0.0000 | 0.00 | 0.6250 / 0.6250 | 485079 / 485079 | 468 | 1489 | 275 | 0 | 913125 |
| 4 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7613 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 351725 / 351725 | 624 | 1630 | 277 | 0 | 906800 |
| 5 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7022 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 215852 / 215852 | 882 | 1865 | 268 | 0 | 892269 |

Representative candidate snapshot:

| route len | candidate route | final score | q conf | warmup | instability ppm / factor | flap penalty | flap count | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |

Feedback events seen from exit:

| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |
| --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 275 | 0 | 967 | 1059 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 275 | 0 | 967 | 1059 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 275 | 0 | 967 | 1059 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 237 | 0 | 808 | 906 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 237 | 0 | 808 | 906 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 237 | 0 | 808 | 906 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 280 | 0 | 955 | 1057 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 280 | 0 | 955 | 1057 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 280 | 0 | 955 | 1057 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 236 | 0 | 840 | 908 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 236 | 0 | 840 | 908 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 236 | 0 | 840 | 908 | 200 |

## Conclusion

The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.
