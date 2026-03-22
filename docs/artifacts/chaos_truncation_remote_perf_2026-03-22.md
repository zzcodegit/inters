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
| direct | 0 | 5 | 0 | 262349 | 174.78/227.72/319.75 | 183.55/211.34/272.69 | 1214.86/1387.94/1741.05 | 192256 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.61/4.12/18.03 | 1023.19/1059.28/1084.14 | 1024.23/1063.92/1086.71 | 246512 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.59/0.68/0.83 | 1091.11/1206.28/1430.98 | 1092.66/1207.47/1432.11 | 219291 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.61/4.39/18.92 | 1524.72/1575.14/1644.24 | 1525.90/1580.07/1645.46 | 166096 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -223.60 | -98.19 | 847.94 | 401.23 | -324.02 | -23.35 | 54256 | 28.22 |
| remote-2hop | -227.04 | -99.70 | 994.94 | 470.79 | -180.47 | -13.00 | 27034 | 14.06 |
| remote-3hop | -223.34 | -98.07 | 1363.80 | 645.32 | 192.13 | 13.84 | -26160 | -13.61 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
