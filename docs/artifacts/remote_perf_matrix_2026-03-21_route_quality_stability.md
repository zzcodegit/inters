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
| direct | 0 | 5 | 0 | 262349 | 200.78/244.87/287.77 | 205.76/272.08/395.99 | 1652.98/1899.23/2225.54 | 139528 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.40/0.48/0.68 | 960.24/1001.36/1063.52 | 960.94/1002.18/1064.30 | 262019 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.35/0.46/0.65 | 987.90/1156.74/1496.78 | 988.72/1157.59/1497.83 | 232576 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.44/0.46/0.51 | 1271.16/1620.02/2345.15 | 1271.96/1620.86/2345.99 | 169332 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -244.39 | -99.80 | 729.29 | 268.04 | -897.05 | -47.23 | 122491 | 87.79 |
| remote-2hop | -244.41 | -99.81 | 884.66 | 325.15 | -741.64 | -39.05 | 93048 | 66.69 |
| remote-3hop | -244.41 | -99.81 | 1347.94 | 495.43 | -278.37 | -14.66 | 29804 | 21.36 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
