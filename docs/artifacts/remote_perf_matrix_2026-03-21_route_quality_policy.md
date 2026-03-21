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
| direct | 0 | 5 | 0 | 262349 | 175.56/197.54/268.45 | 183.27/184.77/186.50 | 1090.62/1281.62/1781.75 | 212279 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.70/4.44/19.19 | 1005.85/1087.67/1270.62 | 1007.46/1092.93/1271.85 | 241460 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.66/0.75/0.88 | 1094.36/1137.60/1227.86 | 1095.58/1138.89/1229.02 | 230539 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.68/0.81/0.98 | 1282.43/1405.28/1505.95 | 1283.83/1406.58/1507.18 | 186935 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -193.10 | -97.75 | 902.90 | 488.65 | -188.69 | -14.72 | 29181 | 13.75 |
| remote-2hop | -196.78 | -99.62 | 952.82 | 515.67 | -142.73 | -11.14 | 18261 | 8.60 |
| remote-3hop | -196.72 | -99.59 | 1220.51 | 660.54 | 124.95 | 9.75 | -25343 | -11.94 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
