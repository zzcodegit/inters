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
| direct | 0 | 5 | 0 | 262349 | 179.30/249.16/297.06 | 184.64/240.52/312.09 | 1547.91/1660.31/1744.68 | 158195 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.56/0.65/0.71 | 7395.89/8071.48/8779.42 | 7397.32/8072.68/8780.48 | 32574 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.59/3.20/13.52 | 10237.28/10643.08/11343.36 | 10238.56/10646.82/11344.45 | 24656 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.62/1.27/2.63 | 10825.24/11321.50/11562.89 | 10826.32/11323.22/11564.50 | 23164 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -248.51 | -99.74 | 7830.96 | 3255.86 | 6412.36 | 386.21 | -125621 | -79.41 |
| remote-2hop | -245.96 | -98.72 | 10402.56 | 4325.05 | 8986.50 | 541.25 | -133539 | -84.41 |
| remote-3hop | -247.89 | -99.49 | 11080.98 | 4607.11 | 9662.91 | 581.99 | -135031 | -85.36 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
