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
| 1 | 200 | 0.42 | 1946.23 | 1946.89 | 262144 | 134647.69 |
| 2 | 200 | 0.48 | 2161.55 | 2162.34 | 262144 | 121231.41 |
| 3 | 200 | 0.40 | 2085.13 | 2085.76 | 262144 | 125682.76 |
| 4 | 200 | 0.41 | 1261.64 | 1262.27 | 262144 | 207676.05 |

Averages: total `1864.32 ms`, TTFB `1863.64 ms`, throughput `147309.48 Bps`.

## Candidate Scoring Evidence

- multi-candidate score events: `5`
- selections with quality metrics attached: `4`
- non-shortest selections observed: `5`
- feedback events received from exit: `21`

| selection | route len | selected route | score | reason | switched | score delta abs | score delta rel % | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | `initial_selection` | `false` | 0.0000 | 0.00 | - | - | - | - | - |
| 2 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7830 | `current_still_best` | `false` | 0.0000 | 0.00 | 301 | 1269 | - | - | - |
| 3 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7713 | `current_still_best` | `false` | 0.0000 | 0.00 | 506 | 1471 | 275 | 0 | 906338 |
| 4 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7709 | `current_still_best` | `false` | 0.0000 | 0.00 | 689 | 1677 | 275 | 0 | 894735 |
| 5 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7102 | `current_still_best` | `false` | 0.0000 | 0.00 | 924 | 1902 | 270 | 0 | 905134 |

Representative candidate snapshot:

| route len | candidate route | final score | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | - | - | - | - | - |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | - | - | - | - | - |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | - | - | - | - | - |

Feedback events seen from exit:

| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |
| --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 275 | 0 | 958 | 1057 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 275 | 0 | 958 | 1057 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 275 | 0 | 958 | 1057 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 240 | 0 | 825 | 906 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 240 | 0 | 825 | 906 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 240 | 0 | 825 | 906 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 275 | 0 | 942 | 1060 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 275 | 0 | 942 | 1060 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 275 | 0 | 942 | 1060 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 250 | 0 | 901 | 1056 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 250 | 0 | 901 | 1056 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 250 | 0 | 901 | 1056 | 200 |

## Conclusion

The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.
