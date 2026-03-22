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
| direct | 0 | 5 | 0 | 262349 | 181.67/230.29/296.18 | 194.03/205.26/210.46 | 1214.24/1368.83/1538.26 | 193450 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.56/0.62/0.69 | 1101.18/1215.85/1488.29 | 1102.33/1216.97/1489.24 | 218063 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.55/0.60/0.64 | 1135.80/1337.70/1445.88 | 1136.96/1338.79/1446.89 | 197273 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.52/0.56/0.59 | 1434.22/1553.67/1636.00 | 1435.38/1554.75/1637.05 | 169018 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -229.68 | -99.73 | 1010.59 | 492.35 | -151.87 | -11.09 | 24614 | 12.72 |
| remote-2hop | -229.70 | -99.74 | 1132.44 | 551.71 | -30.05 | -2.19 | 3823 | 1.98 |
| remote-3hop | -229.74 | -99.76 | 1348.41 | 656.93 | 185.92 | 13.58 | -24431 | -12.63 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
