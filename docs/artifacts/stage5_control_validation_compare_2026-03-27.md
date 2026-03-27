# Stage 5 Control Validation v1

## Scope

- A: baseline v2 exact-route reduced fleet
- B: current control exact-route-compatible branch
- routes: `1-hop`, `2-hop`, `3-hop`, `5-hop`
- each route was run under the existing reset-per-route harness

## Metrics Table

| Route | Version | total avg ms | total p95 ms | total max ms | ack p95 ms | retransmit_rate | inflight avg | inflight max | pacing interval ms | effective cap avg | congestion_events | blocked_by_cap |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `1hop` | `baseline` | 1105.14 | 1109.62 | 1110.58 | 185.20 | 0.0000 | 56.60 | 62.00 | 3.00 | 64.00 | 0 | 0 |
| `1hop` | `control` | 1256.81 | 1698.67 | 1843.55 | 191.80 | 0.0374 | 53.49 | 64.00 | 3.00 | 64.00 | 1 | 6 |
| `2hop` | `baseline` | 1203.99 | 1387.74 | 1406.30 | 192.80 | 0.0000 | 56.00 | 64.00 | 3.00 | 64.00 | 0 | 0 |
| `2hop` | `control` | 1113.31 | 1128.15 | 1131.02 | 189.60 | 0.0000 | 54.20 | 64.00 | 3.00 | 64.00 | 0 | 0 |
| `3hop` | `baseline` | 1386.36 | 1394.32 | 1396.67 | 245.60 | 0.0000 | 54.90 | 63.00 | 4.00 | 64.00 | 0 | 0 |
| `3hop` | `control` | 1475.55 | 1708.74 | 1783.81 | 267.20 | 0.1550 | 53.03 | 64.00 | 4.00 | 64.00 | 2 | 11 |
| `5hop` | `baseline` | 1772.98 | 1779.41 | 1780.46 | 324.60 | 0.0152 | 58.74 | 64.00 | 4.00 | 64.00 | 0 | 0 |
| `5hop` | `control` | 2282.38 | 2678.64 | 2732.22 | 296.00 | 0.2796 | 52.81 | 64.00 | 4.40 | 64.00 | 2 | 173 |

## Route-by-Route Outcome

### `1hop`

- total: `worse` (+13.72%)
- ack p95: `worse`
- retransmit_rate: `worse`
- stall: `worse`

### `2hop`

- total: `better` (-7.53%)
- ack p95: `better`
- retransmit_rate: `neutral`
- stall: `better`

### `3hop`

- total: `worse` (+6.43%)
- ack p95: `worse`
- retransmit_rate: `worse`
- stall: `worse`

### `5hop`

- total: `worse` (+28.73%)
- ack p95: `better`
- retransmit_rate: `worse`
- stall: `better`

## Honest Readout

- `1hop`: total behavior is `worse` versus baseline
- `2hop`: total behavior is `better` versus baseline
- `3hop`: total behavior is `worse` versus baseline
- `5hop`: total behavior is `worse` versus baseline

## Verdict

`CONTROL = REJECTED`

Reasons:

- 1-hop total_time_ms regressed by more than 5% or is unavailable
- 3-hop total_time_ms is worse than baseline
