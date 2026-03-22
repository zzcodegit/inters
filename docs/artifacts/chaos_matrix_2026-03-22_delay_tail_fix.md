# Chaos Matrix Summary: chaos_matrix_2026-03-22_delay_tail_fix

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 167.70 | 0.00 | 169.97 | 0.00 | 197704.44 | 65.00 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 289.77 | 122.07 | 293.67 | 123.70 | 135293.36 | 89.00 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 199.93 | 32.23 | 201.09 | 31.12 | 166177.16 | 83.40 | 0.00 | 0.0000 | 0.00 |
| mild-delay | 1229.43 | 1061.73 | 1237.23 | 1067.25 | 32188.68 | n/a | 0.00 | 0.0000 | 0.00 |
| combined | 1281.51 | 1113.81 | 1287.75 | 1117.78 | 27635.43 | n/a | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client terminal events | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | n/a | response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex32, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | n/a | response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex30, response_transport_terminal_close_observedx15, response_transport_terminal_payload_observedx16 | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | n/a | response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex32, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | n/a | n/a |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | n/a | duplicate_payload_after_local_completionx4, duplicate_payload_after_local_completion_repeatx181, duplicate_terminal_payload_after_local_completionx2, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex32, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_payload_after_local_completionx3 | n/a | n/a |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | n/a | duplicate_payload_after_local_completionx4, duplicate_payload_after_local_completion_repeatx160, duplicate_terminal_payload_after_local_completionx1, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_duplicatex32, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_payload_after_local_completionx3 | duplicate packet detectedx4 | duplicate packet detectedx6 |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx7
- `mild-reorder`: chaos_delayx82
- `mild-delay`: chaos_delayx8492
- `combined`: chaos_delayx9301, chaos_dropx7, chaos_duplicatex10

## First Bottleneck Ranking

- `combined` pressure=5000: late_events=0, client_open_failures=4, exit_open_failures=6, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-reorder` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-loss` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-delay` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00

