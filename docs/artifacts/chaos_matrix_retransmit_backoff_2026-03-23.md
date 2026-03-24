# Chaos Matrix Summary: chaos_matrix_retransmit_backoff_2026-03-23

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 172.58 | 0.00 | 178.06 | 0.00 | 190983.31 | 34.80 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 237.27 | 64.69 | 242.68 | 64.62 | 147326.65 | 37.60 | 2.40 | 0.0632 | 0.00 |
| mild-reorder | 212.68 | 40.10 | 214.58 | 36.52 | 155518.72 | 52.60 | 5.40 | 0.1421 | 0.00 |
| mild-delay | 427.12 | 254.54 | 430.19 | 252.13 | 76854.72 | 182.50 | 0.00 | 0.0000 | 0.00 |
| combined | 748.19 | 575.62 | 755.59 | 577.53 | 45820.70 | 151.67 | 3.80 | 0.1000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client terminal events | exit ack events | client duplicate drops | exit duplicate drops | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx392, inflight_cleanup_by_ack_rangex392, stale_ack_ignoredx32 | n/a | n/a | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx378, inflight_cleanup_by_ack_rangex378, stale_ack_ignoredx54 | n/a | n/a | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx197, inflight_cleanup_by_ack_rangex197, stale_ack_ignoredx264 | n/a | n/a | n/a | n/a |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_close_before_local_completion_queuedx1 | cumulative_ack_advancedx163, inflight_cleanup_by_ack_rangex163, stale_ack_ignoredx314 | n/a | n/a | n/a | n/a |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_close_before_local_completion_queuedx1 | cumulative_ack_advancedx137, inflight_cleanup_by_ack_rangex137, stale_ack_ignoredx445 | duplicate_packet_droppedx5 | duplicate_packet_droppedx2 | n/a | n/a |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx5
- `mild-reorder`: chaos_delayx72
- `mild-delay`: chaos_delayx4366
- `combined`: chaos_delayx5386, chaos_dropx4, chaos_duplicatex7

## First Bottleneck Ranking

- `mild-reorder` pressure=1421: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.1421, window_wait_ms=0.00
- `combined` pressure=999: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.1000, window_wait_ms=0.00
- `mild-loss` pressure=631: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0632, window_wait_ms=0.00
- `mild-delay` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00

