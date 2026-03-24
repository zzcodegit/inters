# Chaos Matrix Summary: chaos_matrix_2026-03-25

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 225.97 | 0.00 | 235.26 | 0.00 | 141684.54 | 78.60 | 0.40 | 0.0105 | 0.00 |
| mild-loss | 270.57 | 44.60 | 275.60 | 40.34 | 122806.42 | 91.20 | 0.60 | 0.0158 | 0.00 |
| mild-reorder | 265.05 | 39.08 | 272.68 | 37.41 | 123749.26 | 95.20 | 6.20 | 0.1632 | 0.00 |
| mild-delay | 971.92 | 745.95 | 982.29 | 747.03 | 36234.98 | 312.00 | 10.20 | 0.2684 | 0.00 |
| combined | 1025.08 | 799.11 | 1044.68 | 809.42 | 32903.44 | 128.00 | 8.40 | 0.2211 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client terminal events | exit ack events | client duplicate drops | exit duplicate drops | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx392, inflight_cleanup_by_ack_rangex392, stale_ack_ignoredx75 | n/a | n/a | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx389, inflight_cleanup_by_ack_rangex389, stale_ack_ignoredx86 | n/a | n/a | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx263, inflight_cleanup_by_ack_rangex263, stale_ack_ignoredx295 | n/a | n/a | n/a | n/a |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_close_before_local_completion_queuedx1 | cumulative_ack_advancedx139, inflight_cleanup_by_ack_rangex139, stale_ack_ignoredx439 | n/a | n/a | n/a | n/a |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_close_before_local_completion_queuedx2 | cumulative_ack_advancedx131, inflight_cleanup_by_ack_rangex131, stale_ack_ignoredx449 | duplicate_packet_droppedx8 | duplicate_packet_droppedx4 | n/a | n/a |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx6
- `mild-reorder`: chaos_delayx82
- `mild-delay`: chaos_delayx5980
- `combined`: chaos_delayx6167, chaos_dropx5, chaos_duplicatex12

## First Bottleneck Ranking

- `mild-delay` pressure=2684: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.2684, window_wait_ms=0.00
- `combined` pressure=2210: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.2211, window_wait_ms=0.00
- `mild-reorder` pressure=1631: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.1632, window_wait_ms=0.00
- `mild-loss` pressure=157: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0158, window_wait_ms=0.00

