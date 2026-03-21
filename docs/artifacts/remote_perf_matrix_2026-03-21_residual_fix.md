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
| direct | 0 | 5 | 0 | 262349 | 194.78/394.10/875.89 | 188.31/214.16/256.09 | 1377.23/1512.89/1893.78 | 175676 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.58/4.86/21.70 | 1060.34/1158.51/1246.14 | 1082.80/1163.96/1247.24 | 225793 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.50/0.58/0.68 | 1071.40/1420.94/2063.54 | 1072.42/1422.00/2064.61 | 194327 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.46/0.51/0.60 | 1619.69/1971.23/2290.02 | 1620.61/1972.20/2291.19 | 135705 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -389.24 | -98.77 | 944.35 | 440.95 | -348.93 | -23.06 | 50116 | 28.53 |
| remote-2hop | -393.52 | -99.85 | 1206.78 | 563.49 | -90.89 | -6.01 | 18650 | 10.62 |
| remote-3hop | -393.58 | -99.87 | 1757.07 | 820.44 | 459.31 | 30.36 | -39972 | -22.75 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
