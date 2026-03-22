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
| direct | 0 | 5 | 0 | 262349 | 194.61/246.63/294.85 | 198.00/565.91/1931.17 | 1431.97/1924.11/3218.80 | 148644 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.69/0.78/0.85 | 1036.49/1123.05/1239.18 | 1037.82/1124.44/1240.63 | 234014 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.70/7.25/22.34 | 1265.94/2431.60/3061.23 | 1278.33/2439.59/3062.69 | 118965 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.72/0.81/0.90 | 1802.77/5340.44/8868.31 | 1804.02/5346.52/8869.65 | 68686 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -245.85 | -99.68 | 557.14 | 98.45 | -799.67 | -41.56 | 85370 | 57.43 |
| remote-2hop | -239.38 | -97.06 | 1865.68 | 329.68 | 515.48 | 26.79 | -29680 | -19.97 |
| remote-3hop | -245.82 | -99.67 | 4774.52 | 843.68 | 3422.41 | 177.87 | -79958 | -53.79 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
