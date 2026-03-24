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
| direct | 0 | 5 | 0 | 262349 | 183.58/188.92/202.99 | 184.67/189.48/191.95 | 1137.35/1281.92/1499.37 | 206592 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.49/0.58/0.66 | 1096.72/1160.16/1212.46 | 1097.92/1161.23/1213.40 | 226106 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.51/0.55/0.64 | 1096.65/1305.75/1523.63 | 1097.60/1306.79/1524.74 | 204401 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.42/0.51/0.66 | 1381.19/1594.04/2016.37 | 1382.21/1595.01/2017.32 | 168066 |
| remote-5hop | 5 | 5 | 0 | 262349 | 0.46/0.63/0.85 | 1883.84/2108.09/2746.09 | 1884.91/2109.14/2746.98 | 126747 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -188.34 | -99.70 | 970.68 | 512.30 | -120.69 | -9.42 | 19514 | 9.45 |
| remote-2hop | -188.37 | -99.71 | 1116.27 | 589.13 | 24.87 | 1.94 | -2191 | -1.06 |
| remote-3hop | -188.41 | -99.73 | 1404.56 | 741.28 | 313.09 | 24.42 | -38527 | -18.65 |
| remote-5hop | -188.29 | -99.67 | 1918.61 | 1012.59 | 827.21 | 64.53 | -79845 | -38.65 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `remote-5hop` endpoint=`127.0.0.1:19185` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
