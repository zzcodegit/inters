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
| direct | 0 | 5 | 0 | 262349 | 183.99/241.56/311.92 | 206.80/233.67/296.64 | 1376.74/1644.13/1865.42 | 161254 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.50/0.57/0.65 | 1092.21/1132.57/1208.37 | 1093.04/1133.59/1209.60 | 231538 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.51/0.61/0.69 | 1147.52/1261.03/1364.48 | 1148.87/1262.30/1365.61 | 208560 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.53/7.48/19.61 | 1383.15/1392.20/1402.84 | 1384.97/1400.27/1407.43 | 187215 |
| remote-5hop | 5 | 5 | 0 | 262349 | 0.53/0.58/0.64 | 1910.46/1981.78/2030.46 | 1911.62/1982.83/2031.42 | 132260 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -240.99 | -99.76 | 898.90 | 384.68 | -510.53 | -31.05 | 70285 | 43.59 |
| remote-2hop | -240.96 | -99.75 | 1027.36 | 439.66 | -381.83 | -23.22 | 47307 | 29.34 |
| remote-3hop | -234.08 | -96.90 | 1158.52 | 495.79 | -243.85 | -14.83 | 25961 | 16.10 |
| remote-5hop | -240.98 | -99.76 | 1748.11 | 748.10 | 338.71 | 20.60 | -28994 | -17.98 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `remote-5hop` endpoint=`127.0.0.1:19185` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
