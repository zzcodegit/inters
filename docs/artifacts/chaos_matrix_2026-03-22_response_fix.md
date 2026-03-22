# Chaos Matrix Summary: chaos_matrix_2026-03-22_response_fix

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 186.67 | 0.00 | 189.67 | 0.00 | 176605.09 | 83.20 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 373.74 | 187.07 | 376.87 | 187.21 | 121754.31 | 80.20 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 201.85 | 15.18 | 203.02 | 13.35 | 165055.44 | 81.20 | 0.00 | 0.0000 | 0.00 |
| mild-delay | 1346.32 | 1159.65 | 1373.81 | 1184.14 | 26528.78 | n/a | 0.00 | 0.0000 | 0.00 |
| combined | 1486.75 | 1300.08 | 1489.06 | 1299.39 | 29176.10 | n/a | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | late_close_after_completionx63, late_payload_after_completionx3 | duplicate packet detectedx2 | duplicate packet detectedx6 |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | late_close_after_completionx57, late_payload_after_completionx3 | duplicate packet detectedx2 | duplicate packet detectedx1 |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | late_close_after_completionx63, late_payload_after_completionx4 | packet too old for replay windowx5 | duplicate packet detectedx4, packet too old for replay windowx4 |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | late_close_after_completionx57, late_payload_after_completionx8 | duplicate packet detectedx1 | duplicate packet detectedx2 |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | late_close_after_completionx63, late_payload_after_completionx6 | duplicate packet detectedx12 | duplicate packet detectedx15 |

## Chaos Actions

- `none`: chaos_delayx2896, chaos_dropx8, chaos_duplicatex8
- `mild-loss`: chaos_delayx3150, chaos_dropx14, chaos_duplicatex4
- `mild-reorder`: chaos_delayx3763, chaos_dropx2, chaos_duplicatex5
- `mild-delay`: chaos_delayx11987, chaos_dropx7, chaos_duplicatex3
- `combined`: chaos_delayx14137, chaos_dropx14, chaos_duplicatex29

## First Bottleneck Ranking

- `combined` pressure=82500: late_events=69, client_open_failures=12, exit_open_failures=15, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-reorder` pressure=73500: late_events=67, client_open_failures=5, exit_open_failures=8, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-delay` pressure=66500: late_events=65, client_open_failures=1, exit_open_failures=2, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-loss` pressure=61500: late_events=60, client_open_failures=2, exit_open_failures=1, retransmit_rate=0.0000, window_wait_ms=0.00

