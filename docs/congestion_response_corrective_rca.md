# Congestion Response Corrective RCA

## Scope

This corrective pass addresses the rejected lightweight congestion-response step from commit `5033259`.

The target was narrow:

- keep Stage 4 behavior green
- keep pacing
- keep retransmit backoff
- keep hard window `64`
- do not introduce full congestion control

## Rejected-Step Symptom

The rejected congestion step did detect long-path pressure, but the `5-hop` path still regressed:

- accepted retransmit-backoff baseline `c5ceea5`: `remote-5hop total = 1780.34 ms`
- rejected congestion step `5033259`: `remote-5hop total = 2109.14 ms`

The corresponding stage sample also showed that the controller entered congestion but did not keep enough pressure off the path:

- `congestion_events = 2`
- `congestion_duration_ms = 180.80`
- `send_blocked_by_effective_cap = 5`

## One Root Cause

The congestion policy was still **point-sample based**, not **path-state based**.

In practice that meant:

- on clean long RTT paths, the baseline stayed too low and could over-detect congestion
- on real loss bursts, the state could enter congestion and then start restoring after only a few loop samples
- even when it entered `SEVERE`, the cap reduction was shallow enough that the path stayed close to the hard window

That combination made the signal real, but not useful enough on the lossy long path.

## Concrete Proof

From the rejected-step `5-hop` stage trace:

- first congestion entries happened with retransmit burst already in progress
- the state then relieved quickly
- average congestion cap stayed near the default path behavior

From the current corrected trace:

- `remote-5hop retransmit_rate`: `1.0108 -> 0.3803`
- `remote-5hop retransmit_trigger_count`: `291.4 -> 109.8`
- `remote-5hop effective_cap_wait_total_ms`: `754.2 -> 313.4`
- `remote-5hop congestion_duration_ms`: `1113.0 -> 646.8`

Source:

- [docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.md](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.md)
- [docs/artifacts/congestion_response_corrective_remote_perf_stage_2026-03-24.md](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_corrective_remote_perf_stage_2026-03-24.md)

## Corrective Policy

This pass turns the previous point-sample policy into a more stateful one:

- adaptive clean-path RTT baseline, instead of pinning to the minimum observed RTT
- RTT-aware minimum hold before relieving congestion
- stronger `SEVERE` cap reduction (`52` instead of `56`)

That keeps good routes mostly untouched while giving the long lossy path a longer and deeper pressure response.

## Honest Outcome

This pass materially shrinks the rejected-step long-path regression:

- `remote-5hop total`: `2109.14 -> 1982.83 ms`

But it does **not** fully beat the accepted retransmit-backoff baseline:

- `1780.34 -> 1982.83 ms`

So this corrective pass improves the rejected congestion step, but it does not fully clear the milestone if the acceptance bar is “beat or match `c5ceea5` on `5-hop` total latency”.
