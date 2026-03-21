# Route-quality Stability Compare

## Summary

| metric | before | after |
| --- | --- | --- |
| series runs | 3 | 3 |
| total selections | 15 | 15 |
| best-score pick ratio % | 100.00 | 100.00 |
| hard switches | 0 | 0 |
| selected route changes | 0 | 0 |
| best-route changes | 0 | 0 |
| avg selected-score stdev | 0.0480 | 0.0412 |
| avg total-time CV % | 33.62 | 19.97 |
| avg TTFB CV % | 33.64 | 19.98 |

## Route Distribution

- before top route: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`
- after top route: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`

## Reading

This comparison focuses on stability signals, not only raw throughput. Lower selected-score stdev and lower latency CV indicate that early noise has less leverage over the selector. If hard switches were already zero in the baseline WAN sample, then the meaningful improvement is reduced score jitter and more confidence-aware reasons rather than an impossible reduction below zero.

