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
| direct | 0 | 5 | 0 | 262349 | 182.85/188.52/197.75 | 182.01/186.08/191.38 | 1272.68/1393.26/1604.38 | 189325 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.34/0.35/0.37 | 7612.60/8836.58/10043.25 | 7613.26/8837.21/10043.83 | 30027 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.40/0.54/0.71 | 6846.65/7495.14/9520.68 | 6847.55/7495.89/9521.33 | 35515 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.36/0.42/0.57 | 8546.67/9274.36/12062.79 | 8547.23/9275.03/12063.53 | 28795 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -188.16 | -99.81 | 8650.50 | 4648.75 | 7443.95 | 534.28 | -159298 | -84.14 |
| remote-2hop | -187.98 | -99.71 | 7309.06 | 3927.87 | 6102.64 | 438.01 | -153810 | -81.24 |
| remote-3hop | -188.09 | -99.78 | 9088.28 | 4884.01 | 7881.77 | 565.71 | -160530 | -84.79 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.

## Interpretation

- The main degradation starts in `TTFB` and carries through to `total_time_ms`; it does not start in the local client ingress connect.
- `remote-2hop` was the best overlay path in this sample, outperforming both `remote-1hop` and `remote-3hop` on average total time and throughput.
- `connect_time_ms` is not an apples-to-apples WAN metric across all scenarios: `direct` measures a public TCP connect to `31.192.232.26:18080`, while `remote-*` measures only the local TCP connect to the client listener on `127.0.0.1`.
