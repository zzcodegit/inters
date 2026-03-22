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
| direct | 0 | 5 | 0 | 262349 | 262.06/299.92/402.20 | 207.24/357.50/889.36 | 1303.01/1586.41/2081.65 | 170135 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.67/0.81/1.04 | 985.37/1359.68/1954.36 | 986.66/1361.12/1955.58 | 205008 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.75/0.90/1.19 | 1105.81/1233.54/1421.20 | 1107.84/1235.33/1422.94 | 214240 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.41/0.45/0.51 | 1276.77/1543.50/2273.93 | 1277.68/1544.44/2274.90 | 177478 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -299.11 | -99.73 | 1002.18 | 280.33 | -225.29 | -14.20 | 34873 | 20.50 |
| remote-2hop | -299.02 | -99.70 | 876.04 | 245.04 | -351.08 | -22.13 | 44105 | 25.92 |
| remote-3hop | -299.47 | -99.85 | 1185.99 | 331.74 | -41.97 | -2.65 | 7343 | 4.32 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
