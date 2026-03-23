# Short-Path ACK Tail RCA

## Scope

This corrective pass is limited to the remaining `1-hop` WAN regression that stayed after the selective inflight-cap correction.

- hard window stays `64`
- effective cap policy stays unchanged
- Stage 4 transport fixes stay unchanged
- routing stays unchanged

## Symptom

On the corrected inflight pass (`b7e70903a4c056f199c46412893e6223a45b54db`), `1-hop` was still materially worse than the accepted pacing baseline even though the cap controller never fired:

- perf avg total: `1151.68 ms -> 1555.92 ms`
- stage avg ACK p95: `190.0 ms -> 397.6 ms`
- stage avg retransmit rate: `0.0000 -> 0.0909`
- effective cap avg: `64 -> 64`
- cap reductions: `0`
- blocked by cap: `0`

That separated the remaining regression from effective-cap throttling.

## One Root Cause

The exit used ACK latency samples derived from `now - first_sent`, not `now - last_sent`.

That means a retransmitted response frame on a short route polluted the ACK summary with the frame's entire lifetime, including timeout/backoff age that no longer represented current path RTT.

Once that happened:

1. ACK summary inflated
2. pacing interval inflated from the polluted ACK summary
3. retransmit interval inflated from the same polluted ACK summary
4. the short route kept a longer response tail after the first retransmit event

So the remaining `1-hop` regression was not caused by the effective cap. It was caused by stale retransmit age leaking into the RTT signal that both pacing and retransmit timing consume.

## Proof

From the previous corrected remote stage trace:

- `1-hop run2`: `retransmit_rate=0.2222`, `ack_p95=578`, `pacing_interval=5`, `cap_avg=64`, `blocked=0`
- `1-hop run3`: `retransmit_rate=0.0972`, `ack_p95=440`, `pacing_interval=5`, `cap_avg=64`, `blocked=0`
- `1-hop run4`: `retransmit_rate=0.1349`, `ack_p95=484`, `pacing_interval=4`, `cap_avg=64`, `blocked=0`

After this fix, the fresh remote stage trace still contains one retransmit-heavy `1-hop` run:

- `1-hop run5`: `retransmit_rate=0.1828`, `ack_p95=188`, `pacing_interval=3`, `cap_avg=64`, `blocked=0`

That is the key causal proof:

- retransmits still happened
- effective cap still did not fire
- but ACK tail no longer inflated to `440-578 ms`
- pacing interval no longer expanded to `4-5 ms`

## Structural Fix

`ReliableStream::apply_ack_with_latency_details()` now samples ACK latency from `last_sent`, not `first_sent`.

This keeps the ACK summary aligned with the latest send attempt that the peer actually acknowledged.

That is intentionally a telemetry/control-signal fix, not a congestion controller:

- no hard-window change
- no full congestion control
- no route policy change
- no Stage 4 lifecycle/replay change

## Regression Coverage

Two targeted tests cover this path:

- `stream_reliable::tests::ack_latency_summary_uses_latest_send_for_retransmitted_frames`
- `roles::exit::tests::response_inflight_discipline_ignores_short_path_retransmit_age_noise`

They prove that retransmitted short-path frames no longer inflate the ACK summary, pacing interval, or retransmit interval just because the original send happened much earlier.
