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
| direct | 0 | 5 | 0 | 262349 | 182.80/236.96/322.31 | 187.36/205.28/253.70 | 1225.83/1744.94/2778.94 | 163280 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.77/0.84/0.89 | 1100.90/1255.28/1841.92 | 1102.35/1256.81/1843.55 | 217357 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.62/0.77/1.01 | 1098.20/1111.81/1129.64 | 1100.09/1113.31/1131.02 | 235486 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.66/0.74/0.83 | 1389.02/1473.86/1782.10 | 1390.90/1475.55/1783.81 | 179353 |
| remote-5hop | 5 | 5 | 0 | 262349 | 0.71/0.84/1.04 | 2000.27/2280.59/2730.53 | 2001.95/2282.38/2732.22 | 116427 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -236.11 | -99.64 | 1050.00 | 511.49 | -488.14 | -27.97 | 54077 | 33.12 |
| remote-2hop | -236.18 | -99.67 | 906.53 | 441.60 | -631.64 | -36.20 | 72206 | 44.22 |
| remote-3hop | -236.22 | -99.69 | 1268.58 | 617.97 | -269.39 | -15.44 | 16073 | 9.84 |
| remote-5hop | -236.12 | -99.65 | 2075.31 | 1010.96 | 537.43 | 30.80 | -46853 | -28.69 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `remote-5hop` endpoint=`127.0.0.1:19185` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
