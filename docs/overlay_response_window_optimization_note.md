# Overlay Response Path Optimization Note

## What Changed

This step fixed one bottleneck only:

- exit response window sizing

It did not also change:

- retransmit interval
- chunk size
- route scoring
- buffering semantics

That isolation matters because the measured improvement can be attributed to the response window change itself.

## What The New Data Says

The new stage summary shows:

- `exit first send delay` is effectively zero
- `window stall` is now around `0.9-1.2s` instead of dominating the whole request
- total overlay request time dropped from `8-11s` range to roughly `1.2-1.5s` range

So the fix did what it was supposed to do:

- removed the dominant artificial BDP cap on the response path
- kept the system stable across local baseline, remote matrix, perf runner, and stage runner

## Best Next Lever

If we keep optimizing the response path, the next highest-value area is:

1. ACK pacing / retransmit tuning on the response path

Why:

- `ack_latency_ms_avg` still tracks total time strongly
- `ack_latency_ms_p95` grows on worse routes
- retransmit rate still rises with hop-count

## Lower-Priority Levers

These matter less than response-path ACK pacing right now:

1. target connect optimization
2. buffered HTTP completion tuning
3. request-side route warmup

Those are no longer the main explanation for the remaining WAN tail in the accepted perf/stage artifacts.
