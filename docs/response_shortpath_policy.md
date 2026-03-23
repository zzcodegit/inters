# Short-Path ACK Signal Policy

## Goal

Preserve accepted pacing and selective inflight discipline, while preventing short-path retransmit age from masquerading as current RTT.

## Policy

For response-path ACK telemetry:

- use `last_sent` as the ACK latency anchor
- do not use `first_sent` for control-signal RTT

This means:

- ACK summary tracks the latest send attempt that the peer acknowledged
- pacing interval stays tied to current ACK cadence
- retransmit interval stays tied to current ACK cadence
- a single retransmit on a short path does not automatically stretch the next pacing interval

## Why This Is Narrow

This pass does not introduce congestion control.

- hard window remains `64`
- effective cap policy remains unchanged
- pacing burst cap remains unchanged
- loss still remains visible through retransmit counters

The only correction is which timestamp feeds the ACK-derived control signal.

## Intended Outcome

- `1-hop` should no longer show heavy ACK-tail inflation without cap activity
- `2-hop` should stay free of the old cap-induced regression
- `3-hop` should keep its benefit over the plain pacing baseline, or at least avoid a large regression

## Proof Sources

- RCA: [response_shortpath_ack_tail_rca.md](../docs/response_shortpath_ack_tail_rca.md)
- compare: [response_shortpath_compare_2026-03-23.md](../docs/artifacts/response_shortpath_compare_2026-03-23.md)
- remote evidence: [response_shortpath_remote_evidence_2026-03-23.md](../docs/artifacts/response_shortpath_remote_evidence_2026-03-23.md)
