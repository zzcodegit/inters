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
| direct | 0 | 5 | 0 | 262349 | 179.33/182.36/184.65 | 184.75/187.33/188.71 | 1120.37/1282.56/1537.17 | 208827 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.57/0.64/0.70 | 1099.50/1114.95/1155.42 | 1100.77/1116.22/1156.40 | 234930 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.40/0.58/0.72 | 1097.08/1810.92/3914.49 | 1098.19/1811.82/3915.53 | 184787 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.52/3.60/15.63 | 1388.26/1483.18/1842.24 | 1389.12/1487.11/1843.27 | 178457 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -181.73 | -99.65 | 927.62 | 495.18 | -166.34 | -12.97 | 26103 | 12.50 |
| remote-2hop | -181.79 | -99.68 | 1623.59 | 866.70 | 529.26 | 41.27 | -24040 | -11.51 |
| remote-3hop | -178.76 | -98.03 | 1295.85 | 691.75 | 204.55 | 15.95 | -30371 | -14.54 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
