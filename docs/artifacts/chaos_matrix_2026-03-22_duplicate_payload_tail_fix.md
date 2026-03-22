# Chaos Matrix Summary: chaos_matrix_2026-03-22_duplicate_payload_tail_fix

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 305.33 | 0.00 | 309.14 | 0.00 | 106928.05 | 156.80 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 339.86 | 34.53 | 345.02 | 35.88 | 99836.02 | 158.00 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 328.76 | 23.42 | 334.19 | 25.05 | 98872.24 | 126.00 | 0.00 | 0.0000 | 0.00 |
| mild-delay | 3173.73 | 2868.40 | 3233.50 | 2924.36 | 14515.28 | n/a | 0.80 | 0.0211 | 0.00 |
| combined | 3210.97 | 2905.64 | 3229.54 | 2920.40 | 16494.09 | n/a | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client terminal events | exit ack events | client duplicate drops | exit duplicate drops | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx392, inflight_cleanup_by_ack_rangex392, stale_ack_ignoredx32 | n/a | n/a | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx387, inflight_cleanup_by_ack_rangex387, stale_ack_ignoredx31 | n/a | n/a | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx145, inflight_cleanup_by_ack_rangex145, stale_ack_ignoredx279 | n/a | n/a | n/a | n/a |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx44, inflight_cleanup_by_ack_rangex44, stale_ack_ignoredx1229 | n/a | n/a | n/a | n/a |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx15, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_close_before_local_completion_queuedx3 | cumulative_ack_advancedx34, inflight_cleanup_by_ack_rangex34, stale_ack_ignoredx1423 | duplicate_packet_droppedx16 | duplicate_packet_droppedx17 | n/a | n/a |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx7
- `mild-reorder`: chaos_delayx83
- `mild-delay`: chaos_delayx14095
- `combined`: chaos_delayx18454, chaos_dropx16, chaos_duplicatex35

## First Bottleneck Ranking

- `mild-delay` pressure=210: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0211, window_wait_ms=0.00
- `mild-reorder` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-loss` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `combined` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00

