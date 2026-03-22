# Chaos Duplicate Noise Compare

## What Changed

This compare isolates the post-delay-tail residual under `combined` chaos:

- before: duplicate ciphertext was surfacing as `open_message_failed`
- after: the same class of traffic is classified as `duplicate_packet_dropped`
- terminal close duplicates remain visible, which proves the fix did not hide duplicate terminal tail by muting logs

## Combined Chaos Before vs After

| signal | before (`delay_tail_fix`) | after (`duplicate_noise_fix`) |
| --- | ---: | ---: |
| `chaos_duplicate` | 10 | 12 |
| client `open_message_failed: duplicate packet detected` | 4 | 0 |
| exit `open_message_failed: duplicate packet detected` | 6 | 0 |
| client `duplicate_packet_dropped` | 0 | 5 |
| exit `duplicate_packet_dropped` | 0 | 7 |
| client `response_transport_terminal_close_duplicate` | 32 | 32 |

Sources:

- before summary: `docs/artifacts/chaos_matrix_2026-03-22_delay_tail_fix.summary.json`
- before stage trace: `docs/artifacts/chaos_matrix_2026-03-22_delay_tail_fix_combined.stage.jsonl`
- after summary: `docs/artifacts/chaos_matrix_2026-03-22_duplicate_noise_fix.summary.json`
- after stage trace: `docs/artifacts/chaos_matrix_2026-03-22_duplicate_noise_fix_combined.stage.jsonl`

## Why This Is The Correct Interpretation

- The duplicate injection did not disappear after the fix. `chaos_duplicate` is still present in the post-fix run.
- Terminal duplicate close observations did not disappear either. `response_transport_terminal_close_duplicate` stayed at `32`.
- The thing that disappeared is only the failure-looking signature:
  - `open_message_failed: duplicate packet detected`

That means the fix reclassified expected duplicate traffic; it did not hide it and it did not depend on a softer chaos profile.

## Targeted Regression

Targeted regression:

- test: `combined_chaos_duplicate_packets_are_classified_without_open_failure`
- log: `docs/artifacts/chaos_duplicate_noise_targeted_regression_2026-03-22.log`
- raw measurements: `docs/artifacts/chaos_duplicate_noise_targeted_regression_2026-03-22.jsonl`
- stage trace: `docs/artifacts/chaos_duplicate_noise_targeted_regression_2026-03-22.stage.jsonl`

Observed counts in the targeted regression:

| signal | count |
| --- | ---: |
| client `open_message_failed: duplicate packet detected` | 0 |
| exit `open_message_failed: duplicate packet detected` | 0 |
| client `duplicate_packet_dropped` | 6 |
| exit `duplicate_packet_dropped` | 4 |
| client `response_transport_terminal_close_duplicate` | 31 |

The regression therefore proves two things at once:

1. duplicate traffic still exists under the same `combined` profile
2. duplicate traffic no longer touches the failure path
