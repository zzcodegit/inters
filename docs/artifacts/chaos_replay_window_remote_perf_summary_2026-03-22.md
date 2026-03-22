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
| direct | 0 | 5 | 0 | 262349 | 176.68/202.94/244.92 | 180.67/228.28/288.22 | 1178.93/1380.88/1460.59 | 191032 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.43/0.50/0.59 | 937.20/956.86/965.52 | 938.04/957.79/966.67 | 273734 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.42/0.52/0.67 | 1019.65/1764.74/2794.75 | 1020.31/1765.58/2795.67 | 183562 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.77/5.73/14.79 | 1528.13/4110.29/5424.32 | 1529.41/4119.57/5439.77 | 78751 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -202.44 | -99.75 | 728.58 | 319.17 | -423.09 | -30.64 | 82702 | 43.29 |
| remote-2hop | -202.42 | -99.74 | 1536.47 | 673.07 | 384.70 | 27.86 | -7470 | -3.91 |
| remote-3hop | -197.21 | -97.18 | 3882.02 | 1700.57 | 2738.69 | 198.33 | -112281 | -58.78 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
