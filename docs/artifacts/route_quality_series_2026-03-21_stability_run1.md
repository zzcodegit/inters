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
| 1 | 200 | 0.67 | 2074.25 | 2075.33 | 262144 | 126314.50 |
| 2 | 200 | 0.72 | 2431.86 | 2433.11 | 262144 | 107740.27 |
| 3 | 200 | 0.78 | 1419.89 | 1420.97 | 262144 | 184482.15 |
| 4 | 200 | 0.75 | 1411.24 | 1412.27 | 262144 | 185618.35 |

Averages: total `1835.42 ms`, TTFB `1834.31 ms`, throughput `151038.82 Bps`.

## Candidate Scoring Evidence

- multi-candidate score events: `5`
- selections with quality metrics attached: `4`
- non-shortest selections observed: `5`
- feedback events received from exit: `21`

| selection | route len | selected route | score | reason | switched | score delta abs | score delta rel % | q conf sel/best | instability sel/best ppm | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | `initial_selection` | `false` | 0.0000 | 0.00 | 0.0000 / 0.0000 | - / - | - | - | - | - | - |
| 2 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7766 | `current_still_best` | `false` | 0.0000 | 0.00 | 0.1250 / 0.1250 | - / - | 1015 | 1977 | - | - | - |
| 3 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7723 | `current_still_best` | `false` | 0.0000 | 0.00 | 0.6250 / 0.6250 | 46776 / 46776 | 1059 | 2004 | 265 | 0 | 816461 |
| 4 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6914 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 162168 / 162168 | 1125 | 2155 | 353 | 2272 | 921637 |
| 5 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7082 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 248500 / 248500 | 867 | 1933 | 324 | 779 | 887802 |

Representative candidate snapshot:

| route len | candidate route | final score | q conf | warmup | instability ppm / factor | flap penalty | flap count | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |

Feedback events seen from exit:

| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `Udp://31.192.232.26:30000` | 279 | 0 | 779 | 907 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 279 | 0 | 779 | 907 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 279 | 0 | 779 | 907 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 265 | 0 | 863 | 1057 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 265 | 0 | 863 | 1057 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 265 | 0 | 863 | 1057 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 311 | 0 | 895 | 1057 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 311 | 0 | 895 | 1057 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 311 | 0 | 895 | 1057 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 284 | 0 | 928 | 1058 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 284 | 0 | 928 | 1058 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 284 | 0 | 928 | 1058 | 200 |

## Conclusion

The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.
