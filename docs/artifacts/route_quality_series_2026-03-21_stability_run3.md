# Remote WAN route-quality scoring

## Setup

- exact route only: `false`
- requested max route length: `3`
- candidate topology: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- local ingress: `127.0.0.1:19383`
- request path: `/perf-262144.bin`
- probe path: `/perf-262144.bin`
- runs: `4`

## Measurements

| run | status | connect ms | TTFB ms | total ms | body bytes | throughput Bps |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | 200 | 0.60 | 2061.82 | 2062.81 | 262144 | 127081.32 |
| 2 | 200 | 0.74 | 2344.97 | 2346.22 | 262144 | 111730.53 |
| 3 | 200 | 0.58 | 1528.05 | 1528.98 | 262144 | 171450.74 |
| 4 | 200 | 0.95 | 1617.60 | 1618.89 | 262144 | 161927.92 |

Averages: total `1889.22 ms`, TTFB `1888.11 ms`, throughput `143047.63 Bps`.

## Candidate Scoring Evidence

- multi-candidate score events: `5`
- selections with quality metrics attached: `4`
- non-shortest selections observed: `5`
- feedback events received from exit: `24`

| selection | route len | selected route | score | reason | switched | score delta abs | score delta rel % | q conf sel/best | instability sel/best ppm | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | `initial_selection` | `false` | 0.0000 | 0.00 | 0.0000 / 0.0000 | - / - | - | - | - | - | - |
| 2 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7706 | `current_still_best` | `false` | 0.0000 | 0.00 | 0.5000 / 0.5000 | 0 / 0 | 993 | 2133 | 305 | 0 | 875515 |
| 3 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7705 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 48920 / 48920 | 1011 | 2088 | 275 | 0 | 909492 |
| 4 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7002 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 60343 / 60343 | 1083 | 2198 | 340 | 0 | 904684 |
| 5 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7128 | `current_still_best` | `false` | 0.0000 | 0.00 | 1.0000 / 1.0000 | 151339 / 151339 | 840 | 1995 | 340 | 0 | 904684 |

Representative candidate snapshot:

| route len | candidate route | final score | q conf | warmup | instability ppm / factor | flap penalty | flap count | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | 0.0000 | 0.0000 | - / 1.0000 | 1.0000 | 0 | - | - | - | - | - |

Feedback events seen from exit:

| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `Udp://31.192.232.26:30000` | 303 | 0 | 832 | 915 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 303 | 0 | 832 | 915 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 303 | 0 | 832 | 915 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 305 | 0 | 1062 | 1213 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 305 | 0 | 1062 | 1213 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 305 | 0 | 1062 | 1213 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 359 | 6897 | 953 | 1057 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 359 | 6897 | 953 | 1057 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 359 | 6897 | 953 | 1057 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 261 | 0 | 841 | 907 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 261 | 0 | 841 | 907 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 261 | 0 | 841 | 907 | 200 |

## Conclusion

The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.
