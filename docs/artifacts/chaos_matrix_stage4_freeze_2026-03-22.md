# Chaos Matrix Summary: chaos_matrix_stage4_freeze_2026-03-22

## Profiles

- `none`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=0
- `mild-loss`: loss_ppm=1000 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=0 skip_packets=24
- `mild-reorder`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=20000 base_delay_ms=0 jitter_ms=0 reorder_extra_delay_ms=40 skip_packets=24
- `mild-delay`: loss_ppm=0 duplicate_ppm=0 reorder_ppm=0 base_delay_ms=4 jitter_ms=2 reorder_extra_delay_ms=0 skip_packets=24
- `combined`: loss_ppm=1000 duplicate_ppm=2000 reorder_ppm=15000 base_delay_ms=8 jitter_ms=6 reorder_extra_delay_ms=30 skip_packets=24

## Exact 3-Hop Compare

| profile | avg TTFB ms | delta vs none | avg total ms | delta vs none | avg throughput Bps | avg ack p95 ms | avg retransmits | avg retransmit rate | avg window wait ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| none | 229.64 | 0.00 | 233.48 | 0.00 | 141165.11 | 116.20 | 0.00 | 0.0000 | 0.00 |
| mild-loss | 270.86 | 41.23 | 274.64 | 41.16 | 130621.94 | 111.60 | 0.00 | 0.0000 | 0.00 |
| mild-reorder | 238.31 | 8.67 | 239.97 | 6.50 | 138358.44 | 116.80 | 0.00 | 0.0000 | 0.00 |
| mild-delay | 1443.97 | 1214.33 | 1445.11 | 1211.64 | 28354.28 | n/a | 0.00 | 0.0000 | 0.00 |
| combined | 1175.98 | 946.34 | 1189.78 | 956.30 | 35835.04 | n/a | 0.00 | 0.0000 | 0.00 |

## Adaptive Selector

| profile | decision events | route changes | decision reasons | selected routes | client late events | client terminal events | exit ack events | client duplicate drops | exit duplicate drops | client open failures | exit open failures |
| --- | ---: | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| none | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19402 -> Udp://127.0.0.1:19403 -> Udp://127.0.0.1:19401x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx392, inflight_cleanup_by_ack_rangex392, stale_ack_ignoredx32 | n/a | n/a | n/a | n/a |
| mild-loss | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19502 -> Udp://127.0.0.1:19503 -> Udp://127.0.0.1:19501x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx388, inflight_cleanup_by_ack_rangex388, stale_ack_ignoredx31 | n/a | n/a | n/a | n/a |
| mild-reorder | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19602 -> Udp://127.0.0.1:19603 -> Udp://127.0.0.1:19601x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx202, inflight_cleanup_by_ack_rangex202, stale_ack_ignoredx222 | n/a | n/a | n/a | n/a |
| mild-delay | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19702 -> Udp://127.0.0.1:19703 -> Udp://127.0.0.1:19701x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16 | cumulative_ack_advancedx52, inflight_cleanup_by_ack_rangex52, stale_ack_ignoredx788 | n/a | n/a | n/a | n/a |
| combined | 5 | 0 | current_still_bestx5 | Udp://127.0.0.1:19802 -> Udp://127.0.0.1:19803 -> Udp://127.0.0.1:19801x5 | n/a | duplicate_close_stream_suppressedx32, response_transport_local_completionx16, response_transport_settlement_completedx16, response_transport_settlement_startedx16, response_transport_terminal_close_observedx16, response_transport_terminal_payload_observedx16, terminal_close_before_local_completion_queuedx2 | cumulative_ack_advancedx43, inflight_cleanup_by_ack_rangex43, stale_ack_ignoredx632 | duplicate_packet_droppedx2 | duplicate_packet_droppedx9 | n/a | n/a |

## Chaos Actions

- `none`: no injected actions
- `mild-loss`: chaos_dropx8
- `mild-reorder`: chaos_delayx79
- `mild-delay`: chaos_delayx9912
- `combined`: chaos_delayx8060, chaos_dropx8, chaos_duplicatex11

## First Bottleneck Ranking

- `mild-reorder` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-loss` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `mild-delay` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00
- `combined` pressure=0: late_events=0, client_open_failures=0, exit_open_failures=0, retransmit_rate=0.0000, window_wait_ms=0.00

