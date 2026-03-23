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
| direct | 0 | 5 | 0 | 262349 | 186.35/190.26/196.10 | 190.93/197.66/214.50 | 1405.14/1914.73/3619.24 | 155758 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.39/0.41/0.43 | 1130.25/1150.94/1205.76 | 1131.05/1151.68/1206.49 | 227750 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.36/0.45/0.50 | 1135.94/1182.46/1326.88 | 1136.81/1183.38/1327.60 | 222295 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.40/0.46/0.55 | 1397.48/1608.21/2339.62 | 1398.26/1609.04/2340.43 | 169481 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -189.85 | -99.78 | 953.29 | 482.30 | -763.05 | -39.85 | 71992 | 46.22 |
| remote-2hop | -189.81 | -99.76 | 984.81 | 498.24 | -731.35 | -38.20 | 66537 | 42.72 |
| remote-3hop | -189.80 | -99.76 | 1410.55 | 713.64 | -305.69 | -15.97 | 13724 | 8.81 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
