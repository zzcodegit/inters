# Delay-Tail Before/After Compare

## Inputs

- Before summary:
  - [chaos_matrix_2026-03-22_replay_fix_clean.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean.summary.json)
- After summary:
  - [chaos_matrix_2026-03-22_delay_tail_fix.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_delay_tail_fix.summary.json)
- Targeted regression:
  - [chaos_delay_tail_targeted_regression_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_targeted_regression_2026-03-22.log)

## Result

`late_payload_after_completion` is no longer present in the current delay-focused chaos profiles.

## Mild Delay

- Before:
  - `late_payload_after_completion = 55`
  - `late_close_after_completion = 48`
- After:
  - `client_late_events = {}`
  - settlement events recorded:
    - `response_transport_local_completion = 16`
    - `response_transport_settlement_started = 16`
    - `response_transport_terminal_payload_observed = 16`
    - `response_transport_terminal_close_observed = 16`
    - `response_transport_settlement_completed = 16`
  - duplicate tail now classified separately:
    - `duplicate_payload_after_local_completion = 4`
    - `duplicate_payload_after_local_completion_repeat = 181`

## Combined

- Before:
  - `late_payload_after_completion = 38`
  - `late_close_after_completion = 42`
- After:
  - `client_late_events = {}`
  - settlement events recorded:
    - `response_transport_local_completion = 16`
    - `response_transport_settlement_started = 16`
    - `response_transport_terminal_payload_observed = 16`
    - `response_transport_terminal_close_observed = 16`
    - `response_transport_settlement_completed = 16`
  - duplicate tail now classified separately:
    - `duplicate_payload_after_local_completion = 4`
    - `duplicate_payload_after_local_completion_repeat = 160`

## Interpretation

- Before this fix, delayed post-completion DATA frames were counted as late payload.
- After this fix, the same tail is modeled explicitly as transport-settlement plus duplicate post-completion response frames.
- `payload_after_local_completion_during_settlement` stays at `0`, so the client is not receiving additional body bytes after local `Content-Length` completion in this sample.

## Notes

- This compare is about correctness and lifecycle semantics, not about claiming lower chaos latency.
- Combined profile still shows duplicate-packet rejects at the session layer in raw logs, but that is a separate limiter and not the root cause fixed in this step.
