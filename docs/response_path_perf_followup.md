# Response Path Perf Follow-Up

## Goal

Measure whether the response-path reliability fix only removes the error log, or whether it also improves the accepted WAN perf matrix in a way that matches the transport hypothesis.

## Inputs

Before-fix artifacts:

- [remote_perf_matrix_2026-03-21.md](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_matrix_2026-03-21.md)
- [remote_perf_stage_matrix_2026-03-21.md](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_stage_matrix_2026-03-21.md)

After-fix artifacts:

- [remote_perf_matrix_2026-03-21_response_fix.md](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_matrix_2026-03-21_response_fix.md)
- [remote_perf_stage_matrix_2026-03-21_response_fix.md](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.md)
- [remote_perf_matrix_2026-03-21_response_fix.jsonl](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_matrix_2026-03-21_response_fix.jsonl)
- [remote_perf_stage_matrix_2026-03-21_response_fix.joined.jsonl](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.joined.jsonl)

## WAN Perf Before vs After

| scenario | avg total before ms | avg total after ms | delta ms | delta % | avg throughput before Bps | avg throughput after Bps | throughput delta % |
| --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | 8072.68 | 6588.14 | -1484.54 | -18.39% | 32574 | 39790 | +22.15% |
| remote-2hop | 10646.82 | 6824.92 | -3821.90 | -35.90% | 24656 | 38415 | +55.80% |
| remote-3hop | 11323.22 | 8837.30 | -2485.92 | -21.95% | 23164 | 29663 | +28.06% |

## Stage Breakdown Before vs After

| scenario | overlay gap before ms | overlay gap after ms | exit tail before ms | exit tail after ms | retransmit before | retransmit after | ack latency before ms | ack latency after ms |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | 7881.21 | 6528.14 | 7681.40 | 6379.20 | 0.0285 | 0.0000 | 215.80 | 177.40 |
| remote-2hop | 11026.88 | 6809.36 | 10790.00 | 6680.60 | 0.2474 | 0.0000 | 304.00 | 185.80 |
| remote-3hop | 11363.22 | 8681.98 | 11125.80 | 8465.80 | 0.2178 | 0.0000 | 311.40 | 238.40 |

## Interpretation

The after-fix numbers match the root-cause hypothesis well:

- target connect stays tiny, so the target path was not the bottleneck before or after
- the largest improvements are on overlay gap and exit/body tail, which is exactly where late response churn would hurt
- retransmit rate drops to `0.0000` across the remote stage matrix
- ACK latency falls materially on all overlay scenarios
- the old error string disappears entirely from the committed perf logs

That combination is much stronger than a pure logging cleanup. It shows the fix changed transport behavior on the response path in the same direction as the earlier RCA predicted.

## Caveat

This fix does not solve every response-path inefficiency:

- the buffered HTTP path still often exits on `buffer_cap_reached`
- cold-start route/session costs are still large in the remote matrix

But the highest-risk residual from the earlier RCA, late response traffic after premature cleanup, is now both contained and observable.
