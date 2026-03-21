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

| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 1 | 0.6363 | 0.6324 | 1.0000 | 1.0063 | 0.250 | 0.089 | - / 1.000 | 1.000 | 0 | 310 | 640 | 140 | 0 | 57142 |
| B | 2 | 0.6099 | 0.6196 | 1.0000 | 1.0044 | 0.250 | 0.089 | - / 1.000 | 1.000 | 0 | 560 | 940 | 240 | 0 | 122448 |
| C | 3 | 0.5832 | 0.6070 | 1.0000 | 1.0008 | 0.250 | 0.089 | - / 1.000 | 1.000 | 0 | 820 | 1320 | 380 | 8000 | 244444 |

### W0: warmed route A becomes current route

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

| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 1 | 0.6506 | 0.6200 | 0.9955 | 1.0541 | 1.000 | 1.000 | 0 / 1.020 | 1.000 | 0 | 540 | 980 | 240 | 0 | 122448 |
| B | 2 | 0.6047 | 0.6199 | 0.9955 | 1.0000 | 0.000 | 0.000 | - / 1.000 | 1.000 | 0 | - | - | - | - | - |

### W1: one lucky cold-start sample does not steal selection

- selected: `A`
- best candidate: `A`
- previous: `A`
- reason: `current_still_best`
- tie-break: `-`
- switched: `false`
- score delta abs: `0.0000`
- score delta ratio: `0.00%`
- required switch margins: abs `0.040`, rel `8.0%`
- hold remaining ms: `-`

| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 1 | 0.6493 | 0.6200 | 0.9935 | 1.0541 | 1.000 | 1.000 | 0 / 1.020 | 1.000 | 0 | 540 | 980 | 240 | 0 | 122448 |
| B | 2 | 0.6071 | 0.6199 | 0.9935 | 1.0060 | 0.250 | 0.089 | - / 1.000 | 1.000 | 0 | 140 | 320 | 90 | 0 | 85714 |

### C: route A degraded, route B should take over

- selected: `B`
- best candidate: `B`
- previous: `A`
- reason: `switch_margin_exceeded`
- tie-break: `-`
- switched: `true`
- score delta abs: `0.3991`
- score delta ratio: `185.98%`
- required switch margins: abs `0.072`, rel `14.5%`
- hold remaining ms: `-`

| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B | 2 | 0.6137 | 0.6196 | 1.0000 | 1.0108 | 0.500 | 0.227 | 65311 / 1.002 | 1.000 | 0 | 542 | 922 | 234 | 0 | 117971 |
| C | 3 | 0.5832 | 0.6070 | 1.0000 | 1.0008 | 0.250 | 0.089 | - / 1.000 | 1.000 | 0 | 820 | 1320 | 380 | 8000 | 244444 |
| A | 1 | 0.2146 | 0.2365 | 0.9394 | 0.8843 | 1.000 | 0.578 | 1051941 / 0.800 | 1.000 | 0 | 2602 | 3307 | 869 | 118260 | 502292 |

### N: noisy route is penalized even when average latency looks competitive

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

| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 1 | 0.6339 | 0.6197 | 0.9865 | 1.0369 | 1.000 | 0.782 | 0 / 1.020 | 1.000 | 0 | 500 | 1000 | 250 | 8000 | 140000 |
| B | 1 | 0.6058 | 0.6197 | 0.9865 | 0.9909 | 1.000 | 0.782 | 1133065 / 0.800 | 1.000 | 0 | 477 | 836 | 275 | 25059 | 268086 |

### H0: current route established

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

| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 1 | 0.6105 | 0.6199 | 0.9805 | 1.0045 | 0.250 | 0.089 | - / 1.000 | 1.000 | 0 | 520 | 900 | 250 | 0 | 122448 |
| B | 2 | 0.5956 | 0.6199 | 0.9805 | 1.0000 | 0.000 | 0.000 | - / 1.000 | 1.000 | 0 | - | - | - | - | - |

### H1: challenger slightly better, hold keeps current route

- selected: `A`
- best candidate: `B`
- previous: `A`
- reason: `hold_time_active`
- tie-break: `-`
- switched: `false`
- score delta abs: `0.0167`
- score delta ratio: `2.75%`
- required switch margins: abs `0.120`, rel `24.0%`
- hold remaining ms: `11000`

| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B | 2 | 0.6232 | 0.6199 | 0.9785 | 1.0485 | 1.000 | 0.782 | 0 / 1.020 | 1.000 | 0 | 430 | 780 | 210 | 0 | 96590 |
| A | 1 | 0.6066 | 0.6199 | 0.9785 | 1.0001 | 0.375 | 0.154 | 491820 / 0.800 | 1.000 | 0 | 658 | 984 | 250 | 0 | 122448 |

### H2: after hold, small delta still stays below hysteresis threshold

- selected: `A`
- best candidate: `B`
- previous: `A`
- reason: `within_hysteresis_margin`
- tie-break: `-`
- switched: `false`
- score delta abs: `0.0165`
- score delta ratio: `2.75%`
- required switch margins: abs `0.076`, rel `15.1%`
- hold remaining ms: `0`

| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| B | 2 | 0.6165 | 0.6199 | 0.9680 | 1.0485 | 1.000 | 0.782 | 0 / 1.020 | 1.000 | 0 | 430 | 780 | 210 | 0 | 96590 |
| A | 1 | 0.6001 | 0.6199 | 0.9680 | 1.0001 | 0.375 | 0.154 | 491820 / 0.800 | 1.000 | 0 | 658 | 984 | 250 | 0 | 122448 |

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

| route | hops | score | base | transport | quality | q conf | warmup | instability ppm / factor | flap penalty | flap count | TTFB ms | total ms | ACK p95 ms | retrans ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A | 1 | 0.6049 | 0.6138 | 0.9855 | 1.0000 | 0.000 | 0.000 | - / 1.000 | 1.000 | 0 | - | - | - | - | - |
| B | 2 | 0.5928 | 0.6138 | 0.9855 | 1.0000 | 0.000 | 0.000 | - / 1.000 | 1.000 | 0 | - | - | - | - | - |

## Result

The controlled policy run proves three things at once: cold-start luck is damped by warmup confidence, noisy routes are penalized by instability, and real degradation still causes a deterministic switch once the challenger is materially better. The anti-flap sub-scenario then shows a smaller B-over-A edge that is recorded, but still suppressed by hold time and hysteresis.
