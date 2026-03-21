# Controlled Route-Quality Causality

## Policy

- switch threshold: absolute `>= 0.040` and relative `>= 8.0%`
- hysteresis margin for ties: absolute `< 0.015` and relative `< 3.0%`
- hold time: `15s`
- emergency switch during hold: absolute `>= 0.100` or relative `>= 20.0%`

## Phases

### A: healthy route A wins

- selected: `A`
- best candidate: `A`
- previous: `-`
- reason: `initial_selection`
- tie-break: `-`
- switched: `false`
- score delta abs: `0.0000`
- score delta ratio: `0.00%`
- required switch margins: abs `0.040`, rel `8.0%`
- hold remaining ms: `-`

| route | hops | score | base | transport | quality | hop factor | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 1 | 0.6435 | 0.6324 | 1.0000 | 1.0176 | 1.0000 | 310 | 640 | 140 | 0 | 57142 |
| B | 2 | 0.6147 | 0.6196 | 1.0000 | 1.0125 | 0.9800 | 560 | 940 | 240 | 0 | 122448 |
| C | 3 | 0.5841 | 0.6070 | 1.0000 | 1.0023 | 0.9600 | 820 | 1320 | 380 | 8000 | 244444 |

### C: route A degraded, route B should take over

- selected: `B`
- best candidate: `B`
- previous: `A`
- reason: `switch_margin_exceeded`
- tie-break: `-`
- switched: `true`
- score delta abs: `0.4299`
- score delta ratio: `221.41%`
- required switch margins: abs `0.040`, rel `8.0%`
- hold remaining ms: `-`

| route | hops | score | base | transport | quality | hop factor | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B | 2 | 0.6240 | 0.6196 | 1.0000 | 1.0278 | 0.9800 | 542 | 922 | 234 | 0 | 117971 |
| C | 3 | 0.5841 | 0.6070 | 1.0000 | 1.0023 | 0.9600 | 820 | 1320 | 380 | 8000 | 244444 |
| A | 1 | 0.1942 | 0.2365 | 0.9394 | 0.8000 | 1.0000 | 2602 | 3307 | 869 | 118260 | 502292 |

### H0: current route established

- selected: `A`
- best candidate: `A`
- previous: `-`
- reason: `initial_selection`
- tie-break: `-`
- switched: `false`
- score delta abs: `0.0000`
- score delta ratio: `0.00%`
- required switch margins: abs `0.040`, rel `8.0%`
- hold remaining ms: `-`

| route | hops | score | base | transport | quality | hop factor | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 1 | 0.6154 | 0.6199 | 0.9805 | 1.0126 | 1.0000 | 520 | 900 | 250 | 0 | 122448 |
| B | 2 | 0.5956 | 0.6199 | 0.9805 | 1.0000 | 0.9800 | - | - | - | - | - |

### H1: challenger slightly better, hold keeps current route

- selected: `A`
- best candidate: `B`
- previous: `A`
- reason: `hold_time_active`
- tie-break: `-`
- switched: `false`
- score delta abs: `0.0181`
- score delta ratio: `2.94%`
- required switch margins: abs `0.100`, rel `20.0%`
- hold remaining ms: `11000`

| route | hops | score | base | transport | quality | hop factor | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B | 2 | 0.6338 | 0.6199 | 0.9785 | 1.0663 | 0.9800 | 450 | 800 | 220 | 0 | 100000 |
| A | 1 | 0.6157 | 0.6199 | 0.9785 | 1.0151 | 1.0000 | 592 | 924 | 250 | 0 | 122448 |

### H2: after hold, small delta still stays below hysteresis threshold

- selected: `A`
- best candidate: `B`
- previous: `A`
- reason: `within_hysteresis_margin`
- tie-break: `-`
- switched: `false`
- score delta abs: `0.0179`
- score delta ratio: `2.94%`
- required switch margins: abs `0.040`, rel `8.0%`
- hold remaining ms: `0`

| route | hops | score | base | transport | quality | hop factor | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B | 2 | 0.6270 | 0.6199 | 0.9680 | 1.0663 | 0.9800 | 450 | 800 | 220 | 0 | 100000 |
| A | 1 | 0.6091 | 0.6199 | 0.9680 | 1.0151 | 1.0000 | 592 | 924 | 250 | 0 | 122448 |

### T: tie-break prefers shorter route

- selected: `A`
- best candidate: `A`
- previous: `-`
- reason: `initial_selection`
- tie-break: `shorter_route_tie_break`
- switched: `false`
- score delta abs: `0.0000`
- score delta ratio: `0.00%`
- required switch margins: abs `0.040`, rel `8.0%`
- hold remaining ms: `-`

| route | hops | score | base | transport | quality | hop factor | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 1 | 0.6049 | 0.6138 | 0.9855 | 1.0000 | 1.0000 | - | - | - | - | - |
| B | 2 | 0.5928 | 0.6138 | 0.9855 | 1.0000 | 0.9800 | - | - | - | - | - |

## Result

The controlled policy run proves causality: route A wins when healthy, route A is then degraded with ACK/retransmit/stall plus failures, its score drops below route B, and the selector switches to B. The anti-flap sub-scenario then shows a smaller B-over-A edge that is real enough to make B the best-score candidate, but still too small to justify a switch during hold time or after hysteresis.
