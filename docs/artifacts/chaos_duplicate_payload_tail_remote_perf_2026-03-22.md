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
| direct | 0 | 5 | 0 | 262349 | 227.10/256.35/286.21 | 193.37/307.25/635.35 | 1528.10/3196.71/6523.08 | 112461 |
| remote-1hop | 1 | 5 | 0 | 262349 | 0.35/0.46/0.74 | 958.21/981.09/1044.12 | 959.32/981.85/1044.80 | 267257 |
| remote-2hop | 2 | 5 | 0 | 262349 | 0.40/0.44/0.52 | 995.04/1351.06/2716.60 | 995.69/1351.88/2717.47 | 226905 |
| remote-3hop | 3 | 5 | 0 | 262349 | 0.35/0.44/0.64 | 1234.32/1259.17/1269.72 | 1235.42/1260.17/1270.66 | 208044 |

## Degradation vs direct

| scenario | avg connect delta ms | avg connect delta % | avg TTFB delta ms | avg TTFB delta % | avg total delta ms | avg total delta % | avg throughput delta Bps | avg throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | -255.90 | -99.82 | 673.84 | 219.31 | -2214.86 | -69.29 | 154796 | 137.65 |
| remote-2hop | -255.91 | -99.83 | 1043.81 | 339.73 | -1844.83 | -57.71 | 114444 | 101.76 |
| remote-3hop | -255.91 | -99.83 | 951.93 | 309.82 | -1936.54 | -60.58 | 95584 | 84.99 |

## Topology proof

- `direct` endpoint=`31.192.232.26:18080` route_chain=`client -> 31.192.232.26:18080 -> target`
- `remote-1hop` endpoint=`127.0.0.1:19181` route_chain=`client -> 31.192.232.26:30000 -> target`
- `remote-2hop` endpoint=`127.0.0.1:19182` route_chain=`client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop` endpoint=`127.0.0.1:19183` route_chain=`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Current relay/exit runtime still keeps effectively single-session state, so the perf runner uses the same explicit reset hook between remote scenarios as the accepted WAN matrix.
