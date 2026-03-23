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
| direct | 0 | 5 | 0 | 262349 | 182.37/184.84/187.74 | 187.65/194.42/208.19 | 1119.44/1863.26/4581.88 | 189588 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.56/0.62/0.68 | 1365.96/1554.75/1744.96 | 1367.01/1555.92/1746.06 | 169597 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.88/0.92/0.97 | 1115.13/1209.77/1388.43 | 1116.85/1212.74/1390.04 | 217912 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.67/0.77/0.93 | 1394.42/1405.83/1445.39 | 1396.03/1407.29/1446.63 | 186312 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -184.22 | -99.67 | 1360.33 | 699.69 | -307.34 | -16.49 | -19991 | -10.54 |
| remote-2hop | -183.91 | -99.50 | 1015.36 | 522.25 | -650.52 | -34.91 | 28324 | 14.94 |
| remote-3hop | -184.07 | -99.58 | 1211.42 | 623.10 | -455.98 | -24.47 | -3276 | -1.73 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
