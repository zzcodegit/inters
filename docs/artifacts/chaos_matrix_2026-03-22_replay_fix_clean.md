# Chaos Matrix Summary: chaos_matrix_2026-03-22_replay_fix_clean

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 161.54 | 0.00 | 163.58 | 0.00 | 202909.48 | 53.60 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 194.74 | 33.19 | 197.18 | 33.61 | 178788.41 | 57.20 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 173.68 | 12.14 | 174.33 | 10.75 | 192354.97 | 76.40 | 0.00 | 0.0000 | 0.00 |
| mild-delay | 429.17 | 267.62 | 434.47 | 270.89 | 80377.58 | n/a | 0.00 | 0.0000 | 0.00 |
| combined | 606.19 | 444.65 | 611.91 | 448.34 | 56931.23 | n/a | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | late_close_after_completionx48 | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | late_close_after_completionx48 | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | late_close_after_completionx48 | n/a | n/a |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | late_close_after_completionx48, late_payload_after_completionx55 | n/a | n/a |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | late_close_after_completionx42, late_payload_after_completionx38 | duplicate packet detectedx3 | duplicate packet detectedx4 |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx8
- `mild-reorder`: chaos_delayx76
- `mild-delay`: chaos_delayx4676
- `combined`: chaos_delayx4867, chaos_dropx4, chaos_duplicatex7

## First Bottleneck Ranking

- `mild-delay` pressure=103000: late_events=103, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `combined` pressure=83500: late_events=80, client_open_failures=3, exit_open_failures=4, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-reorder` pressure=48000: late_events=48, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-loss` pressure=48000: late_events=48, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00

