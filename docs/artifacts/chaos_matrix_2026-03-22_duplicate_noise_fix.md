# Chaos Matrix Summary: chaos_matrix_2026-03-22_duplicate_noise_fix

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 159.86 | 0.00 | 161.79 | 0.00 | 206693.68 | 59.60 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 196.32 | 36.46 | 198.44 | 36.64 | 177150.23 | 53.00 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 170.19 | 10.33 | 170.84 | 9.05 | 195906.18 | 67.00 | 0.00 | 0.0000 | 0.00 |
| mild-delay | 342.01 | 182.15 | 347.47 | 185.67 | 95811.17 | n/a | 0.00 | 0.0000 | 0.00 |
| combined | 369.81 | 209.95 | 372.39 | 210.60 | 90292.05 | n/a | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client terminal events | client duplicate drops | exit duplicate drops | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | n/a | response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex32, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | n/a | n/a | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | n/a | response_transport_local_completionx16, response_transport_settlement_completedx15, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex30, response_transport_terminal_close_observedx15, response_transport_terminal_payload_observedx16 | n/a | n/a | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | n/a | response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex32, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | n/a | n/a | n/a | n/a |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | n/a | response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex32, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | n/a | n/a | n/a | n/a |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | n/a | response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex32, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | duplicate_packet_droppedx5 | duplicate_packet_droppedx7 | n/a | n/a |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx6
- `mild-reorder`: chaos_delayx84
- `mild-delay`: chaos_delayx4036
- `combined`: chaos_delayx4379, chaos_dropx2, chaos_duplicatex12

## First Bottleneck Ranking

- `mild-reorder` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-loss` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-delay` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `combined` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00

