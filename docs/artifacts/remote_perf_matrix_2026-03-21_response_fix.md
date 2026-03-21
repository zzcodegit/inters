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
| direct | 0 | 5 | 0 | 262349 | 178.57/182.89/192.03 | 181.73/183.57/185.92 | 1071.86/1244.14/1480.89 | 213393 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.36/0.45/0.53 | 6564.85/6587.29/6610.27 | 6565.64/6588.14/6611.24 | 39790 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.35/0.38/0.41 | 6750.95/6824.20/6965.86 | 6751.65/6824.92/6966.72 | 38415 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.34/0.43/0.66 | 8818.55/8836.66/8868.52 | 8819.42/8837.30/8869.14 | 29663 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -182.44 | -99.75 | 6403.72 | 3488.49 | 5344.00 | 429.53 | -173602 | -81.35 |
| remote-2hop | -182.51 | -99.79 | 6640.63 | 3617.54 | 5580.78 | 448.56 | -174978 | -82.00 |
| remote-3hop | -182.46 | -99.77 | 8653.09 | 4713.85 | 7593.16 | 610.31 | -183729 | -86.10 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
