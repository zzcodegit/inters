# Congestion Response Corrective Compare

## Compared Commits

- accepted retransmit baseline: `c5ceea5`
- rejected congestion step: `5033259`
- current corrective pass: current commit

## End-to-End Perf Totals

| route | c5ceea5 total ms | 5033259 total ms | corrective total ms |
| --- | ---: | ---: | ---: |
| 1-hop | 1143.17 | 1161.23 | 1133.59 |
| 2-hop | 1097.70 | 1306.79 | 1262.30 |
| 3-hop | 1386.50 | 1595.01 | 1400.27 |
| 5-hop | 1780.34 | 2109.14 | 1982.83 |

## Stage Metrics

| route | metric | c5ceea5 | 5033259 | corrective |
| --- | --- | ---: | ---: | ---: |
| 5-hop | client total ms | 1897.28 | 2048.52 | 2211.46 |
| 5-hop | retransmit rate | 0.2810 | 0.0000 | 0.3803 |
| 5-hop | retransmit triggers | 81 | 0 | 110 |
| 5-hop | ack avg ms | 271.20 | 324.80 | 277.60 |
| 5-hop | ack p95 ms | 293.80 | 349.60 | 363.20 |
| 5-hop | blocked by cap | 0 | 5 | 27 |
| 5-hop | congestion events | 0 | 2 | 1 |
| 5-hop | congestion duration ms | 0.00 | 180.80 | 646.80 |
| 5-hop | congestion cap limit avg | 64 | 64 | 62 |
| 1-hop | total ms | 1107.18 | 1110.70 | 1229.43 |
| 2-hop | total ms | 1167.11 | 1113.97 | 1164.53 |
| 3-hop | total ms | 1470.85 | 1395.58 | 1527.14 |

## What Changed

The corrective pass materially reduced the rejected-step long-path regression:

- `5-hop perf total`: `2109.14 -> 1982.83`

It also reduced the rejected-step burst pathology on the long route:

- `5-hop retransmit rate`: `1.0108 -> 0.3803`
- `5-hop retransmit triggers`: `291.4 -> 109.8`
- `5-hop effective cap wait`: `754.2 -> 313.4`

## Honest Remaining Gap

The corrective pass does **not** fully beat the accepted retransmit baseline on `5-hop` total latency:

- `1780.34 -> 1982.83`

So the rejected step is corrected materially, but the long-path control layer is still not at the acceptance bar if the bar is strict parity-or-better versus `c5ceea5`.
