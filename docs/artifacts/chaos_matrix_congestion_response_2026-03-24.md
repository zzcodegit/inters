# Chaos Matrix Summary: chaos_matrix_congestion_response_2026-03-24

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 180.50 | 0.00 | 189.46 | 0.00 | 178506.19 | 52.40 | 0.40 | 0.0105 | 0.00 |
| mild-loss | 225.18 | 44.69 | 235.44 | 45.98 | 154094.49 | 54.20 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 197.41 | 16.92 | 198.77 | 9.31 | 169615.48 | 71.40 | 0.40 | 0.0105 | 0.00 |
| mild-delay | 445.70 | 265.20 | 447.69 | 258.23 | 75120.07 | 194.50 | 0.00 | 0.0000 | 0.00 |
| combined | 453.71 | 273.21 | 455.79 | 266.33 | 74743.74 | 167.00 | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client terminal events | exit ack events | client duplicate drops | exit duplicate drops | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx392, inflight_cleanup_by_ack_rangex392, stale_ack_ignoredx34 | n/a | n/a | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx389, inflight_cleanup_by_ack_rangex389, stale_ack_ignoredx32 | n/a | n/a | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx162, inflight_cleanup_by_ack_rangex162, stale_ack_ignoredx264 | n/a | n/a | n/a | n/a |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_close_before_local_completion_queuedx2 | cumulative_ack_advancedx172, inflight_cleanup_by_ack_rangex172, stale_ack_ignoredx313 | n/a | n/a | n/a | n/a |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_close_before_local_completion_queuedx1 | cumulative_ack_advancedx138, inflight_cleanup_by_ack_rangex138, stale_ack_ignoredx339 | duplicate_packet_droppedx2 | duplicate_packet_droppedx2 | n/a | n/a |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx4
- `mild-reorder`: chaos_delayx92
- `mild-delay`: chaos_delayx4440
- `combined`: chaos_delayx4180, chaos_dropx2, chaos_duplicatex4

## First Bottleneck Ranking

- `mild-reorder` pressure=105: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0105, window_wait_ms=0.00
- `mild-loss` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-delay` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `combined` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00

