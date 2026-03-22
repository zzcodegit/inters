# Chaos Replay-Window RCA

## What failed first after the lifecycle fix

After the `MISSING response channel` race was removed, the next repeatable transport failure under chaos was not route selection and not response truncation. It was session-level anti-replay rejection during reorder bursts.

The strongest evidence came from the pre-fix local chaos summary in [docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix.summary.json):

- `mild-reorder`: client `packet too old for replay window x5`, exit `packet too old for replay window x6`
- `combined`: exit `packet too old for replay window x1`

Those rejects happened in `SessionCrypto::open_message()` before decrypt, so the response path could still finish successfully, but the transport was already discarding legitimate delayed packets that had only arrived late.

## Root cause

The old replay window in [src/session.rs](/C:/neinternet/vpnnode/src/session.rs) tracked only `64` sequence numbers with a single `u64` bitmap.

That was too narrow for the chaos profiles because:

- session sequence numbers cover all encrypted messages, not only response payloads
- the reorder profile can delay packets by `40 ms`
- within that delay, request frames, response frames, ACKs and control packets can advance `highest` far enough that a still-legitimate delayed packet falls outside the `64`-packet window

So the transport was classifying bounded reorder as stale replay.

## Structural fix

The fix in [src/session.rs](/C:/neinternet/vpnnode/src/session.rs):

- widened the replay bitmap from `64` to `512` packets
- kept the window bounded, so obviously stale traffic is still rejected
- preserved duplicate rejection inside the active window
- added structured reject context directly into the error string:
  - `seq`
  - `highest`
  - `behind`
  - `window`

This means the stage traces and raw logs now show exactly why a reject happened without changing the normal transport path when no reject occurs.

## Regression coverage

New coverage:

- [src/session.rs](/C:/neinternet/vpnnode/src/session.rs): unit tests for
  - late reordered packet accepted within the expanded window
  - unseen packet beyond the window still rejected
- [tests/chaos_matrix.rs](/C:/neinternet/vpnnode/tests/chaos_matrix.rs): `reorder_chaos_does_not_reject_packets_inside_replay_window`

The full chaos runner in [scripts/run_chaos_matrix.ps1](/C:/neinternet/vpnnode/scripts/run_chaos_matrix.ps1) was also tightened to execute only the matrix collector test, so targeted regressions no longer pollute the per-profile matrix summaries.

## What changed in practice

From [docs/artifacts/chaos_replay_window_compare_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_replay_window_compare_2026-03-22.md):

- `mild-reorder`: replay-window rejects went from `11` to `0`
- `combined`: replay-window rejects went from `1` to `0`

The current clean chaos summary is in [docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean.md).

## What is now the next remaining limiter

After this fix, the highest-pressure local chaos profile is no longer reorder-related open-message rejection.

In the clean post-fix matrix:

- `mild-reorder`: no client or exit open-message failures
- `mild-delay`: highest pressure due to `late_payload_after_completion` and a large TTFB/total increase
- `combined`: still green, with only duplicate-packet rejects remaining, which are expected under intentional duplication

So the next honest chaos bottleneck is delay-driven response-tail behavior, not replay-window intolerance.

## Remote follow-up

Remote validation stayed green after the replay-window change:

- WAN matrix: [docs/artifacts/chaos_replay_window_remote_wan_matrix_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_replay_window_remote_wan_matrix_2026-03-22.log)
- perf: [docs/artifacts/chaos_replay_window_remote_perf_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_replay_window_remote_perf_2026-03-22.log)
- stage: [docs/artifacts/chaos_replay_window_remote_perf_stage_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_replay_window_remote_perf_stage_2026-03-22.log)

Replay-window reject counts in those remote wrapper and stage artifacts are captured in [docs/artifacts/chaos_replay_window_remote_reject_counts_2026-03-22.txt](/C:/neinternet/vpnnode/docs/artifacts/chaos_replay_window_remote_reject_counts_2026-03-22.txt).
