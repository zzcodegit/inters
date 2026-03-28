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
| direct | 0 | 5 | 0 | 262349 | 182.40/186.81/198.79 | 186.79/189.36/193.23 | 1131.72/1283.48/1510.35 | 206629 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.38/0.42/0.47 | 1099.09/1155.88/1238.95 | 1099.91/1156.76/1239.72 | 227287 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.54/0.58/0.61 | 1094.58/1169.99/1380.74 | 1095.63/1170.98/1381.58 | 225549 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.43/0.49/0.55 | 1380.90/1393.11/1407.94 | 1382.01/1394.20/1409.18 | 188033 |
| remote-5hop | 5 | 5 | 0 | 262349 | 0.43/0.55/0.75 | 1789.32/1956.19/2198.02 | 1790.66/1957.21/2198.81 | 134703 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -186.39 | -99.77 | 966.52 | 510.41 | -126.72 | -9.87 | 20658 | 10.00 |
| remote-2hop | -186.23 | -99.69 | 980.63 | 517.86 | -112.50 | -8.77 | 18920 | 9.16 |
| remote-3hop | -186.32 | -99.74 | 1203.75 | 635.69 | 110.72 | 8.63 | -18597 | -9.00 |
| remote-5hop | -186.26 | -99.71 | 1766.83 | 933.05 | 673.73 | 52.49 | -71926 | -34.81 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `remote-5hop` endpoint=`127.0.0.1:19185` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
