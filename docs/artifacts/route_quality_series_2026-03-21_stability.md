# Remote WAN Route-Quality Series

## Summary

- series runs: `3`
- total selections: `15`
- selections that picked current best-score route: `15`
- best-score pick ratio: `100.00%`
- total switches: `0`
- selected route changes: `0`
- best-score leader changes: `0`
- average selected-score stdev per run: `0.0412`
- average total-time CV per run: `19.97%`
- average TTFB CV per run: `19.98%`
- note: selection counts include the initial readiness/probe stream for each independent run

Selection distribution:

- `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`: `15`

Decision reasons:

- `current_still_best`: `12`
- `initial_selection`: `3`

## Per-run Distribution

| run | selections | best-score picks | switches | selected route changes | best-route changes | score stdev | total CV % | TTFB CV % | selected route distribution | decision reasons |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| route_quality_series_2026-03-21_stability_run1.client | 5 | 5/5 | 0 | 0 | 0 | 0.0427 | 23.84 | 23.85 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` x5 | `current_still_best` x4, `initial_selection` x1 |
| route_quality_series_2026-03-21_stability_run2.client | 5 | 5/5 | 0 | 0 | 0 | 0.0416 | 18.48 | 18.49 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` x5 | `current_still_best` x4, `initial_selection` x1 |
| route_quality_series_2026-03-21_stability_run3.client | 5 | 5/5 | 0 | 0 | 0 | 0.0393 | 17.59 | 17.60 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` x5 | `current_still_best` x4, `initial_selection` x1 |

## First Selection Snapshot Per Run

### route_quality_series_2026-03-21_stability_run1.client

- selected route: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`
- best-score route: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`
- decision reason: `initial_selection`
- switched: `False`
- score delta abs / rel: `0.0000` / `0.00%`
- selected quality confidence / best quality confidence: `0.000` / `0.000`
- selected instability ppm / best instability ppm: `None` / `None`
| route len | candidate route | final score | q conf | warmup | instability ppm | flap penalty | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | 0.0 | 0.0 | None | 1.0 | None | None | None | None |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | 0.0 | 0.0 | None | 1.0 | None | None | None | None |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | 0.0 | 0.0 | None | 1.0 | None | None | None | None |

### route_quality_series_2026-03-21_stability_run2.client

- selected route: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`
- best-score route: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`
- decision reason: `initial_selection`
- switched: `False`
- score delta abs / rel: `0.0000` / `0.00%`
- selected quality confidence / best quality confidence: `0.000` / `0.000`
- selected instability ppm / best instability ppm: `None` / `None`
| route len | candidate route | final score | q conf | warmup | instability ppm | flap penalty | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | 0.0 | 0.0 | None | 1.0 | None | None | None | None |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | 0.0 | 0.0 | None | 1.0 | None | None | None | None |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | 0.0 | 0.0 | None | 1.0 | None | None | None | None |

### route_quality_series_2026-03-21_stability_run3.client

- selected route: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`
- best-score route: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`
- decision reason: `initial_selection`
- switched: `False`
- score delta abs / rel: `0.0000` / `0.00%`
- selected quality confidence / best quality confidence: `0.000` / `0.000`
- selected instability ppm / best instability ppm: `None` / `None`
| route len | candidate route | final score | q conf | warmup | instability ppm | flap penalty | recent total ms | recent ACK p95 ms | retransmit ppm | stall ppm |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 3 | `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000` | 0.6720 | 0.0 | 0.0 | None | 1.0 | None | None | None | None |
| 1 | `Udp://31.192.232.26:30000` | 0.4200 | 0.0 | 0.0 | None | 1.0 | None | None | None | None |
| 2 | `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000` | 0.4116 | 0.0 | 0.0 | None | 1.0 | None | None | None | None |

## Conclusion

The WAN series shows whether route choice is caused by the live quality scoreboard rather than pure shortest-path bias. When `decision_reason` is `current_still_best` or `switch_margin_exceeded`, the selected route equals the top-scored route. When `decision_reason` is `hold_time_active` or `within_hysteresis_margin`, the scoreboard still records the challenger, but the selector intentionally suppresses a flap.

