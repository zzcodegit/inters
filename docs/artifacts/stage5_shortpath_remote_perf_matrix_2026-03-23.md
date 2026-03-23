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
| direct | 0 | 5 | 0 | 262349 | 177.16/234.02/302.50 | 183.12/222.60/296.50 | 1178.81/1414.52/1542.82 | 187241 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.53/3.80/16.57 | 1093.14/1117.62/1195.69 | 1094.55/1122.00/1196.76 | 233896 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.46/0.60/0.79 | 1109.40/1195.63/1375.04 | 1110.49/1196.78/1376.29 | 220670 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.56/3.57/15.32 | 1386.71/1441.16/1566.68 | 1387.84/1445.42/1567.90 | 181746 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -230.22 | -98.37 | 895.02 | 402.08 | -292.52 | -20.68 | 46655 | 24.92 |
| remote-2hop | -233.42 | -99.75 | 973.04 | 437.13 | -217.74 | -15.39 | 33429 | 17.85 |
| remote-3hop | -230.45 | -98.47 | 1218.56 | 547.43 | 30.90 | 2.18 | -5495 | -2.93 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
