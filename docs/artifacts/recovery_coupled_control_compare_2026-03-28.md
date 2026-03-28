# Recovery-Coupled Control Compare

## Inputs

- baseline v2 exact-route reduced fleet: commit `f2847ba693da9be0d0da05df9563bbf458f7701a`
- rejected control validation: commit `5997da36d2af1ff77f7ba9d750fbe9a1b8b8e7db`
- current recovery-coupled candidate: branch `codex/stage5-recovery-coupled-control-v1`

Current end-to-end totals come from:

- `docs/artifacts/recovery_coupled_control_remote_perf_matrix_2026-03-28.md`

Current stage metrics come from the **latest** `stream_complete` per `stage-remote-*-runN` in:

- `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.exit.jsonl`

This avoids mixing the successful collection with earlier retry artifacts in the raw trace.

## Route Table

| Route | Baseline v2 total avg ms | Rejected control total avg ms | Recovery-coupled total avg ms | Delta vs baseline | Delta vs rejected |
| --- | ---: | ---: | ---: | ---: | ---: |
| `1-hop` | `1105.14` | `1256.81` | `1156.76` | `+4.67%` | `-7.96%` |
| `2-hop` | `1203.99` | `1113.31` | `1170.98` | `-2.74%` | `+5.18%` |
| `3-hop` | `1386.36` | `1475.55` | `1394.20` | `+0.57%` | `-5.51%` |
| `5-hop` | `1772.98` | `2282.38` | `1957.21` | `+10.39%` | `-14.25%` |

## Stage Metrics

| Route | Baseline v2 ack p95 ms | Rejected control ack p95 ms | Recovery-coupled ack p95 ms | Baseline retransmit | Rejected retransmit | Recovery-coupled retransmit | Rejected blocked-by-cap | Recovery-coupled blocked-by-cap |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `1-hop` | `185.2` | `191.8` | `195.0` | `0.0000` | `0.0374` | `0.0000` | `6` | `0` |
| `2-hop` | `192.8` | `189.6` | `185.6` | `0.0000` | `0.0000` | `0.0000` | `0` | `0` |
| `3-hop` | `245.6` | `267.2` | `244.8` | `0.0000` | `0.1550` | `0.0000` | `11` | `0` |
| `5-hop` | `324.6` | `296.0` | `334.4` | `0.0152` | `0.2796` | `0.0374` | `173` | `0` |

## Current Exact-Route Readout

### `1-hop`

- better than rejected control
- within the `+5%` acceptance guard vs baseline v2
- no measured retransmit in the latest stage set
- no blocked-by-cap

### `2-hop`

- no regression vs baseline v2
- cleaner than rejected control
- no measured retransmit in the latest stage set
- no blocked-by-cap

### `3-hop`

- essentially flat vs baseline v2 (`+0.57%`)
- materially cleaner than rejected control
- no measured retransmit in the latest stage set
- no blocked-by-cap

### `5-hop`

- still above baseline v2
- materially below rejected control
- retransmit pressure is materially lower than rejected control (`0.2796 -> 0.0374`)
- blocked-by-cap fallout from the rejected control is gone (`173 -> 0`)

## Causal Notes

The current model no longer produces the harmful throttle signature seen in the rejected control:

- no measured `blocked_by_cap` on `1/2/3/5-hop`
- no measured congestion duration on the latest `1/2/3/5-hop` run set

But the current exact measured `5-hop` bad stream still completed with:

- `retransmit_rate = 0.1869`
- `retransmit_trigger_count = 54`
- `congestion_events = 0`

So the latest measured long-path improvement versus the rejected control is **not yet** a clean proof that active recovery control itself carried the bad stream. The measured bad `5-hop` stream was cleaner than the rejected control, but not because a sustained recovery state obviously stayed engaged in that stream.

## Acceptance Check

### Passes

- `1-hop` does not regress more than `5%` vs baseline v2
- `2-hop` does not regress
- `3-hop` does not regress materially
- no hop-specific tuning is present
- the harmful blocked-by-cap signature from the rejected control is removed on the exact measured run set

### Fails

- `5-hop` is still above baseline v2
- the benefit on the measured bad `5-hop` stream is not yet causally complete, because the measured bad stream itself finished with `congestion_events = 0`
- the full local chaos matrix is still not green because `response_recovery_guard_avoids_harmful_throttling_on_moderate_route` fails inside the full run

## Current Verdict

`CONTROL = REJECTED`
