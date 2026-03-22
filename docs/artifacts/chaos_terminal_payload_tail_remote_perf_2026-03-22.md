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
| direct | 0 | 5 | 0 | 262349 | 206.68/266.33/296.64 | 204.03/341.59/621.70 | 1340.55/2289.13/4152.61 | 137080 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.64/0.74/0.81 | 1092.98/1113.58/1129.19 | 1094.38/1115.04/1130.46 | 235139 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.59/0.75/0.83 | 1073.11/1177.30/1451.78 | 1074.63/1178.79/1453.47 | 225063 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.68/0.76/0.86 | 1492.91/3323.28/9904.45 | 1507.17/3327.27/9905.69 | 131382 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -265.58 | -99.72 | 771.99 | 226.00 | -1174.09 | -51.29 | 98059 | 71.53 |
| remote-2hop | -265.58 | -99.72 | 835.71 | 244.65 | -1110.34 | -48.50 | 87983 | 64.18 |
| remote-3hop | -265.57 | -99.72 | 2981.69 | 872.88 | 1038.14 | 45.35 | -5698 | -4.16 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
