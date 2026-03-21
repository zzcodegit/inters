# Residual Response Path Stage Compare

## Overlay-window snapshot vs residual-fix snapshot

| scenario | total avg ms before | total avg ms after | retransmit rate before | retransmit rate after | ACK p95 before | ACK p95 after | window stall before | window stall after | exit/body tail before | exit/body tail after |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | 1205.89 | 1451.97 | 0.0214 | 0.0554 | 311.2 | 415.2 | 893.8 | 1101.4 | 1026.8 | 1208.8 |
| remote-2hop | 1402.47 | 1469.85 | 0.1058 | 0.0000 | 390.6 | 371.2 | 1051.8 | 933.6 | 1177.2 | 1056.2 |
| remote-3hop | 1590.06 | 3955.00 | 0.1820 | 0.3388 | 394.0 | 1164.8 | 1223.4 | 2697.4 | 1328.0 | 2811.4 |

## Readout

- `remote-2hop` is the cleanest improvement case: retransmit pressure disappears and the tail shrinks.
- `remote-1hop` and especially `remote-3hop` got worse in the new live WAN sample, which means the current limiter is no longer just local transport policy.
- The old probe-stream missing-channel noise is still eliminated completely, so stability improved even where latency did not.
