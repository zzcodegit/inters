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
| direct | 0 | 5 | 0 | 262349 | 198.87/262.44/302.06 | 206.57/277.43/400.24 | 1534.51/1717.60/2027.83 | 153937 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.48/0.68/0.85 | 1202.04/2087.89/3697.03 | 1203.36/2089.22/3698.17 | 148745 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.50/5.48/25.14 | 1094.54/1151.20/1263.62 | 1095.38/1157.11/1264.57 | 227101 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.53/1.17/3.25 | 1391.27/1491.25/1565.04 | 1392.38/1492.93/1566.22 | 175935 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -261.76 | -99.74 | 1810.46 | 652.58 | 371.62 | 21.64 | -5192 | -3.37 |
| remote-2hop | -256.95 | -97.91 | 873.77 | 314.95 | -560.48 | -32.63 | 73164 | 47.53 |
| remote-3hop | -261.27 | -99.55 | 1213.82 | 437.52 | -224.67 | -13.08 | 21998 | 14.29 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
