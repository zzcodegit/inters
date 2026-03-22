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
| direct | 0 | 5 | 0 | 262349 | 181.35/183.46/184.97 | 181.70/185.90/192.59 | 1100.59/1216.00/1281.19 | 216385 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.37/0.43/0.48 | 936.95/1022.07/1309.47 | 937.63/1022.84/1310.12 | 260561 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.57/0.65/0.77 | 1081.53/1257.84/1412.41 | 1082.76/1258.96/1413.31 | 210258 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.35/0.37/0.39 | 1249.89/2195.03/4177.13 | 1250.46/2195.75/4177.94 | 142376 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -183.03 | -99.77 | 836.17 | 449.79 | -193.16 | -15.88 | 44176 | 20.42 |
| remote-2hop | -182.81 | -99.65 | 1071.94 | 576.62 | 42.97 | 3.53 | -6127 | -2.83 |
| remote-3hop | -183.09 | -99.80 | 2009.13 | 1080.76 | 979.75 | 80.57 | -74009 | -34.20 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
