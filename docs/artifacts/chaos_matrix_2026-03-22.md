# Chaos Matrix Summary: chaos_matrix_2026-03-22

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 172.52 | 0.00 | 175.56 | 0.00 | 192649.75 | 72.40 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 228.87 | 56.35 | 232.66 | 57.11 | 157756.40 | 85.00 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 224.24 | 51.72 | 225.48 | 49.93 | 147306.58 | 97.40 | 0.00 | 0.0000 | 0.00 |
| mild-delay | 1224.94 | 1052.42 | 1236.39 | 1060.83 | 29342.41 | n/a | 0.00 | 0.0000 | 0.00 |
| combined | 1171.11 | 998.59 | 1216.77 | 1041.21 | 31630.96 | n/a | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | late_close_after_completionx48 | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | late_close_after_completionx48 | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | late_close_after_completionx48, late_payload_after_completionx1 | packet too old for replay windowx6 | packet too old for replay windowx3 |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | late_close_after_completionx48, late_payload_after_completionx5 | n/a | n/a |
| combined | 0 | 0 | n/a | n/a | late_close_after_completionx18, late_payload_after_completionx3 | duplicate packet detectedx5 | duplicate packet detectedx12 |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx5
- `mild-reorder`: chaos_delayx77
- `mild-delay`: chaos_delayx8074
- `combined`: chaos_delayx6893, chaos_dropx9, chaos_duplicatex17

## First Bottleneck Ranking

- `mild-reorder` pressure=53500: late_events=49, client_open_failures=6, exit_open_failures=3, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-delay` pressure=53000: late_events=53, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-loss` pressure=48000: late_events=48, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `combined` pressure=29500: late_events=21, client_open_failures=5, exit_open_failures=12, retransmit_rate=0.0000, window_wait_ms=0.00

