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
| 1 | 200 | 0.44 | 1811.50 | 1812.47 | 262144 | 144633.20 |
| 2 | 200 | 0.36 | 1980.75 | 1981.55 | 262144 | 132292.34 |
| 3 | 200 | 0.46 | 1136.59 | 1137.52 | 262144 | 230452.70 |
| 4 | 200 | 0.59 | 1157.18 | 1158.13 | 262144 | 226350.20 |
| 5 | 200 | 0.40 | 1167.45 | 1168.38 | 262144 | 224365.55 |
| 6 | 200 | 0.38 | 1140.30 | 1141.12 | 262144 | 229725.46 |

Averages: total `1399.86 ms`, TTFB `1398.96 ms`, throughput `197969.91 Bps`.

## Candidate Scoring Evidence

- multi-candidate score events: `7`
- selections with quality metrics attached: `6`
- non-shortest selections observed: `7`
- feedback events received from exit: `27`

| selection | route len | selected route | score | recent TTFB ms | recent total ms | recent ACK p95 ms | recent retransmit ppm | recent stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | - | - | - | - | - |
| 2 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7669 | 756 | 1640 | 249 | 0 | 939560 |
| 3 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7678 | 798 | 1679 | 248 | 0 | 947842 |
| 4 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7054 | 936 | 1820 | 252 | 0 | 926987 |
| 5 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7259 | 723 | 1614 | 252 | 0 | 926987 |
| 6 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7459 | 582 | 1476 | 252 | 0 | 855839 |
| 7 | 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.7640 | 484 | 1382 | 246 | 0 | 850574 |

Representative candidate snapshot:

| route len | candidate route | final score | recent TTFB ms | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | - | - | - | - | - |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | - | - | - | - | - |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | - | - | - | - | - |

Feedback events seen from exit:

| route len | route | ACK p95 ms | retransmit ppm | window wait total ms | stream duration ms | http code |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | `Udp://31.192.232.26:30000` | 229 | 0 | 719 | 906 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 229 | 0 | 719 | 906 | 200 |
| 1 | `Udp://31.192.232.26:30000` | 229 | 0 | 719 | 906 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 249 | 0 | 855 | 910 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 249 | 0 | 855 | 910 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 249 | 0 | 855 | 910 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 226 | 0 | 779 | 909 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 226 | 0 | 779 | 909 | 200 |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 226 | 0 | 779 | 909 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 248 | 0 | 865 | 905 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 248 | 0 | 865 | 905 | 200 |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 248 | 0 | 865 | 905 | 200 |

## Conclusion

The client no longer selects purely by availability or shortest path. Each candidate now carries recent end-to-end timing plus exit-side response-path feedback: ACK latency, retransmit rate, and window/backpressure stall. The selected route is the candidate with the best blended score at selection time, and the raw stage trace in the committed artifact shows both the full candidate list and the winning score for each selection event.
