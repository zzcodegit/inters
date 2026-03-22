# Chaos Matrix Summary: chaos_matrix_2026-03-22_terminal_tail_ack_gap_fix

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 222.87 | 0.00 | 226.36 | 0.00 | 145620.12 | 111.80 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 270.45 | 47.58 | 274.54 | 48.18 | 131273.07 | 113.80 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 233.73 | 10.86 | 236.24 | 9.88 | 140855.30 | 116.60 | 0.00 | 0.0000 | 0.00 |
| mild-delay | 1101.61 | 878.74 | 1110.06 | 883.70 | 36995.42 | n/a | 0.00 | 0.0000 | 0.00 |
| combined | 1328.27 | 1105.40 | 1338.37 | 1112.01 | 30188.45 | n/a | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client terminal events | exit ack events | client duplicate drops | exit duplicate drops | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx392, inflight_cleanup_by_ack_rangex392, stale_ack_ignoredx32 | n/a | n/a | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx389, inflight_cleanup_by_ack_rangex389, stale_ack_ignoredx31 | n/a | n/a | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx225, inflight_cleanup_by_ack_rangex225, stale_ack_ignoredx199 | n/a | n/a | n/a | n/a |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | n/a | duplicate_close_stream_suppressedx32, duplicate_payload_after_local_completionx5, duplicate_payload_after_local_completion_repeatx175, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_payload_after_local_completionx4 | cumulative_ack_advancedx57, inflight_cleanup_by_ack_rangex57, stale_ack_ignoredx741 | n/a | n/a | n/a | n/a |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | n/a | duplicate_close_stream_suppressedx32, duplicate_payload_after_local_completionx5, duplicate_payload_after_local_completion_repeatx141, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_close_before_local_completion_queuedx1, terminal_payload_after_local_completionx2 | cumulative_ack_advancedx44, inflight_cleanup_by_ack_rangex44, stale_ack_ignoredx695 | duplicate_packet_droppedx4 | duplicate_packet_droppedx8 | n/a | n/a |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx6
- `mild-reorder`: chaos_delayx83
- `mild-delay`: chaos_delayx9092
- `combined`: chaos_delayx8485, chaos_dropx8, chaos_duplicatex12

## First Bottleneck Ranking

- `mild-reorder` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-loss` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-delay` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `combined` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00

