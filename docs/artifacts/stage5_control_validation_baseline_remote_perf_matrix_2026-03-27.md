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
| direct | 0 | 5 | 0 | 262349 | 181.47/212.26/308.12 | 187.19/207.02/280.29 | 1126.70/1934.44/4072.55 | 165616 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.39/0.50/0.67 | 1098.88/1104.27/1109.72 | 1099.65/1105.14/1110.58 | 237207 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.37/0.54/0.73 | 1098.83/1202.95/1405.39 | 1099.68/1203.99/1406.30 | 220178 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.40/0.51/0.61 | 1380.88/1385.53/1395.90 | 1381.77/1386.36/1396.67 | 189091 |
| remote-5hop | 5 | 5 | 0 | 262349 | 0.44/0.49/0.54 | 1766.00/1772.17/1779.59 | 1766.86/1772.98/1780.46 | 147856 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -211.76 | -99.76 | 897.25 | 433.42 | -829.30 | -42.87 | 71591 | 43.23 |
| remote-2hop | -211.71 | -99.74 | 995.93 | 481.08 | -730.45 | -37.76 | 54562 | 32.94 |
| remote-3hop | -211.75 | -99.76 | 1178.51 | 569.28 | -548.08 | -28.33 | 23475 | 14.17 |
| remote-5hop | -211.76 | -99.77 | 1565.16 | 756.05 | -161.46 | -8.35 | -17760 | -10.72 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `remote-5hop` endpoint=`127.0.0.1:19185` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
