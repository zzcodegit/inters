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
| direct | 0 | 5 | 0 | 262349 | 177.26/190.94/213.55 | 179.90/217.88/290.57 | 1160.55/1281.10/1420.49 | 205986 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.39/0.40/0.41 | 933.55/1269.61/2551.71 | 934.22/1270.34/2552.36 | 241418 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.41/0.48/0.63 | 981.25/1059.65/1184.53 | 981.99/1060.47/1185.37 | 248243 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.38/0.45/0.54 | 1214.98/1234.76/1255.36 | 1215.81/1235.58/1256.38 | 212192 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -190.54 | -99.79 | 1051.73 | 482.70 | -10.76 | -0.84 | 35432 | 17.20 |
| remote-2hop | -190.46 | -99.75 | 841.77 | 386.34 | -220.63 | -17.22 | 42257 | 20.51 |
| remote-3hop | -190.49 | -99.76 | 1016.88 | 466.71 | -45.53 | -3.55 | 6206 | 3.01 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
