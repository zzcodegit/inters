# Stage 5 Control Root Cause Teardown

Reference inputs:

- baseline v2 compare: [stage5_control_validation_compare_2026-03-27.md](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_compare_2026-03-27.md)
- baseline exit trace: [stage5_control_validation_baseline_remote_perf_stage_2026-03-27.exit.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_baseline_remote_perf_stage_2026-03-27.exit.jsonl)
- control exit trace: [stage5_control_validation_control_remote_perf_stage_2026-03-27.exit.jsonl](/C:/neinternet/vpnnode_stage5_control_validation_v1/docs/artifacts/stage5_control_validation_control_remote_perf_stage_2026-03-27.exit.jsonl)

## Root Cause 1

The congestion signal is out of phase with the real retransmit burst.

Evidence:

- `1-hop`, control stream `7`
  - first non-zero retransmit signal appears while congestion is still `none`:
    - `timestamp=1774604628697`
    - `level=none`
    - `retransmit_rate=0.3902`
    - `retransmit_burst=53`
    - `ack_latency_ms_p95=250`
    - `baseline_rtt_ms=183`
  - congestion enters only later:
    - `timestamp=1774604628710`
    - `level=severe`
    - `cap_limit_frames=52`
    - `pacing_extra_ms=2`
  - blocked-by-cap happens after that:
    - `timestamp=1774604628874`
    - `effective_cap=52`
    - `waited_ms=164`
  - result on this stream:
    - `stream_duration_ms=1662`
    - `retransmit_rate=0.2215`
    - `retransmit_trigger_count=64`
    - `send_blocked_by_effective_cap=3`

- `3-hop`, control stream `4`
  - first retransmit signal appears with congestion still `none`:
    - `timestamp=1774605775633`
    - `level=none`
    - `retransmit_rate=0.2368`
    - `ack_latency_ms_p95=258`
    - `baseline_rtt_ms=234`
  - there is no congestion entry and no blocked-by-cap on this stream
  - result on this stream:
    - `stream_duration_ms=1665`
    - `retransmit_rate=0.2180`
    - `retransmit_trigger_count=63`
    - `send_blocked_by_effective_cap=0`

- `5-hop`, control streams `3/5/6/7`
  - congestion does fire, but the state window is still out of phase with loss recovery
  - example `stream 3`:
    - enter severe:
      - `timestamp=1774606351518`
      - `retransmit_rate=0.1515`
      - `retransmit_burst=5`
      - `cap_limit_frames=52`
      - `pacing_extra_ms=2`
    - later the trace reports `relieved` / `no_congestion` while retransmit pressure is already much worse:
      - `timestamp=1774606351783`
      - `retransmit_rate=1.2500`
      - `retransmit_burst=15`
      - `ack_latency_ms_p95=340`
    - and again:
      - `timestamp=1774606352164`
      - `retransmit_rate=2.4337`
      - `ack_latency_ms_p95=742`

This means the signal does not share the same time horizon as retransmit recovery:

- on short and medium paths it reacts after the burst already started, or never reacts
- on long lossy paths it oscillates between `entered` and `relieved` while retransmit pressure is still rising

## Root Cause 2

When the signal does fire, the response cuts throughput, not loss recovery.

Evidence:

- `5-hop` route-level outcome versus baseline v2:
  - `total_time_ms avg: 1772.98 -> 2282.38`
  - `ack p95: 324.60 -> 296.00`
  - `window_stall_ms: 597.0 -> 93.4`
  - `retransmit_rate: 0.0152 -> 0.2796`
  - `blocked_by_cap: 0 -> 173`

- `5-hop` control stream examples:
  - `stream 5`
    - `stream_duration_ms=1661`
    - `retransmit_rate=1.4533`
    - `retransmit_trigger_count=420`
    - `send_blocked_by_effective_cap=85`
    - first blocked event arrives at `effective_cap=60`, then severe cap `52`
  - `stream 6`
    - `stream_duration_ms=1815`
    - `retransmit_rate=1.3646`
    - `retransmit_trigger_count=393`
    - `send_blocked_by_effective_cap=31`
  - `stream 7`
    - `stream_duration_ms=2267`
    - `retransmit_rate=0.2222`
    - `retransmit_trigger_count=64`
    - `send_blocked_by_effective_cap=8`

- `1-hop` control stream `7`
  - once congestion enters, the sender is throttled by cap `52`
  - retransmit has already started
  - the response adds `waited_ms=164` on the first block event
  - final result is longer completion time without any system-level gain

This is the key mismatch:

- queue-facing metrics improve
  - lower ACK tail on `5-hop`
  - lower window stall on `5-hop`
- but loss-facing metrics get worse
  - retransmit rate rises sharply
  - retransmit trigger count rises sharply
- so completion takes longer even though stall looks better

The model is reducing fresh send pressure after loss already exists, while retransmit logic itself remains on its own timing.

## Per-Route Readout

### 1-hop

- retransmit appears only in one bad stream, but that one stream is enough to move route total from `1105.14` to `1256.81`
- causality on the bad stream is:
  - retransmit burst starts first
  - congestion enters second
  - blocked-by-cap appears third
- this is a false positive on a short route: control turns a transient bad run into a longer one

### 3-hop

- retransmit grows from `0.0000` to `0.1550`
- congestion events do not help because the bad stream never enters congestion at all
- ACK p95 rises from `245.60` to `267.20`
- this is a missed-detection path, not a successful control path

### 5-hop

- congestion clearly detects pressure and clearly changes behavior
- but the changed behavior is not useful enough:
  - it lowers ACK tail and stall
  - it does not lower retransmit pressure
  - it inserts `blocked_by_cap`
  - total latency still gets worse by `+28.73%`

## Direct Answers

### 1. Does control react too early or too late?

Too late on `1-hop` and `5-hop` onset, and effectively not at all on the bad `3-hop` stream.

### 2. Is cap reduction too weak or too aggressive?

It is applied in the wrong place:

- too aggressive for short-path transient loss, because it throttles `1-hop`
- too weak against long-path loss bursts, because `5-hop` retransmit rate keeps rising even while cap is reduced

### 3. Do pacing and congestion conflict?

Yes, in the current model they conflict during recovery.

Base pacing is already shaping the stream. Congestion then adds extra pacing and cap reduction after loss has started, which lowers queue pressure but slows completion.

### 4. Is retransmit logic synchronized with congestion?

No.

The traces show retransmit pressure beginning before congestion enters, and in `5-hop` the retransmit rate remains high while congestion is already active.

## Why This Breaks The System

The current layer is trying to solve congestion by throttling send throughput after loss is already present, instead of changing the loss-recovery path itself.

That creates two bad outcomes:

- on short and medium routes, transient loss gets amplified into longer completion time
- on long lossy routes, ACK tail and stall improve, but retransmit pressure stays high, so total latency still gets worse

## Verdict

`CURRENT CONTROL MODEL = PARTIALLY WRONG`

It is not purely decorative, because it does change `5-hop` behavior.

But it is wrong at the level that matters for acceptance:

- the signal is temporally misaligned with retransmit bursts
- the response acts on throughput more than on loss recovery
