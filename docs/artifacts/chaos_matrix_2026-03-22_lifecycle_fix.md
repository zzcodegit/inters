# Chaos Matrix Summary: chaos_matrix_2026-03-22_lifecycle_fix

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 164.39 | 0.00 | 166.20 | 0.00 | 201286.12 | 53.00 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 200.77 | 36.38 | 202.97 | 36.76 | 175712.73 | 56.40 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 179.91 | 15.52 | 180.64 | 14.44 | 184493.91 | 77.20 | 0.00 | 0.0000 | 0.00 |
| mild-delay | 374.04 | 209.65 | 379.47 | 213.27 | 88302.34 | n/a | 0.00 | 0.0000 | 0.00 |
| combined | 402.03 | 237.64 | 405.86 | 239.66 | 84523.46 | n/a | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | late_close_after_completionx63, late_payload_after_completionx3 | n/a | duplicate packet detectedx1 |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | late_close_after_completionx57, late_payload_after_completionx3 | duplicate packet detectedx2 | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | late_close_after_completionx63, late_payload_after_completionx16 | duplicate packet detectedx3, packet too old for replay windowx5 | duplicate packet detectedx3, packet too old for replay windowx6 |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | late_close_after_completionx60, late_payload_after_completionx26 | duplicate packet detectedx1 | duplicate packet detectedx1 |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | late_close_after_completionx60, late_payload_after_completionx10 | duplicate packet detectedx4 | duplicate packet detectedx5, packet too old for replay windowx1 |

## Chaos Actions

- `none`: chaos_delayx1658, chaos_dropx3, chaos_duplicatex1
- `mild-loss`: chaos_delayx1844, chaos_dropx9, chaos_duplicatex2
- `mild-reorder`: chaos_delayx1930, chaos_dropx2, chaos_duplicatex6
- `mild-delay`: chaos_delayx6055, chaos_dropx3, chaos_duplicatex2
- `combined`: chaos_delayx6399, chaos_dropx9, chaos_duplicatex9

## First Bottleneck Ranking

- `mild-reorder` pressure=87500: late_events=79, client_open_failures=8, exit_open_failures=9, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-delay` pressure=87000: late_events=86, client_open_failures=1, exit_open_failures=1, retransmit_rate=0.0000, window_wait_ms=0.00
- `combined` pressure=75000: late_events=70, client_open_failures=4, exit_open_failures=6, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-loss` pressure=61000: late_events=60, client_open_failures=2, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00

