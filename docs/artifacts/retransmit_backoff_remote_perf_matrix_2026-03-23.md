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
| direct | 0 | 5 | 0 | 262349 | 180.85/182.64/184.99 | 184.32/190.28/198.22 | 1127.80/1820.02/3472.19 | 167461 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.41/0.44/0.49 | 1095.64/1142.34/1294.26 | 1096.37/1143.17/1295.19 | 230258 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.44/0.48/0.61 | 1088.61/1096.77/1105.92 | 1089.39/1097.70/1106.75 | 238822 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.35/0.40/0.45 | 1379.69/1385.72/1392.17 | 1380.61/1386.50/1392.81 | 189071 |
| remote-5hop | 5 | 5 | 0 | 262349 | 0.36/0.42/0.51 | 1756.33/1779.57/1822.75 | 1757.01/1780.34/1823.59 | 147271 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -182.21 | -99.76 | 952.06 | 500.34 | -676.85 | -37.19 | 62797 | 37.50 |
| remote-2hop | -182.16 | -99.74 | 906.49 | 476.39 | -722.32 | -39.69 | 71361 | 42.61 |
| remote-3hop | -182.24 | -99.78 | 1195.44 | 628.24 | -433.52 | -23.82 | 21610 | 12.90 |
| remote-5hop | -182.22 | -99.77 | 1589.29 | 835.23 | -39.68 | -2.18 | -20190 | -12.06 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `remote-5hop` endpoint=`127.0.0.1:19185` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
