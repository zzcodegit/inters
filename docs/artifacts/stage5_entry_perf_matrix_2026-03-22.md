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
| direct | 0 | 5 | 0 | 262349 | 207.79/261.69/356.78 | 207.84/258.89/352.58 | 1541.38/1821.51/1970.58 | 145002 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.59/0.62/0.63 | 1043.43/1210.38/1429.47 | 1044.62/1211.49/1430.72 | 218779 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.72/4.46/19.30 | 1039.81/1075.05/1089.03 | 1041.07/1080.18/1103.16 | 242778 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.73/0.85/1.11 | 1350.34/1500.08/1615.09 | 1351.53/1501.64/1616.63 | 175198 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -261.08 | -99.76 | 951.49 | 367.52 | -610.03 | -33.49 | 73777 | 50.88 |
| remote-2hop | -257.24 | -98.30 | 816.16 | 315.25 | -741.33 | -40.70 | 97776 | 67.43 |
| remote-3hop | -260.84 | -99.68 | 1241.18 | 479.42 | -319.87 | -17.56 | 30196 | 20.82 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
