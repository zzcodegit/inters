// Wrapper integration test so `cargo test --tests baseline` picks up the suite.
//
// Cargo discovers integration tests as `tests/*.rs` (top-level), not nested
// directories. The actual suite lives under `tests/baseline/`.
#[path = "baseline/baseline.rs"]
mod baseline_suite;

