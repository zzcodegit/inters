# Remote WAN performance matrix

## Setup

- payload path: `/perf-262144.bin`
- direct public path: `31.192.232.26:18080`
- remote exit path: `31.192.232.26:30000 -> target`
- runs per scenario: `5`
- exact route only: `true`

## Summary

| scenario | route | success | errors | resp bytes avg | connect ms min/avg/max | TTFB ms min/avg/max | total ms min/avg/max | avg throughput Bps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | 5 | 0 | 262349 | 184.75/231.82/297.42 | 207.69/307.73/364.69 | 1321.73/1548.86/1729.21 | 171033 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.57/1.34/4.22 | 7123.10/9193.20/10542.61 | 7124.16/9195.18/10543.82 | 29235 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.58/0.73/1.18 | 7918.13/8323.69/8601.17 | 7919.15/8324.90/8602.28 | 31515 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.57/0.68/0.79 | 10542.37/11283.22/12141.50 | 10543.62/11284.50/12142.85 | 23279 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -230.48 | -99.42 | 8885.47 | 2887.42 | 7646.32 | 493.67 | -141798 | -82.91 |
| remote-2hop | -231.09 | -99.69 | 8015.96 | 2604.87 | 6776.04 | 437.49 | -139519 | -81.57 |
| remote-3hop | -231.14 | -99.71 | 10975.49 | 3566.60 | 9735.64 | 628.57 | -147755 | -86.39 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
