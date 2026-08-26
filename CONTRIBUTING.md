# Contributing

Keep changes small enough that one conformance case explains the behavior being added.

Before submitting a change, run:

```console
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

New checker behavior needs at least one passing and one failing fixture when both outcomes
are meaningful. Add a named test for each fixture to `tests/conformance.rs`, which makes it
individually runnable in VS Code's Test Explorer. Use Cargo's test-name filter to isolate a
case and `BLESS=1` to update its baseline. Do not bless a broad suite without reviewing the
resulting diff.

Avoid copying implementation structure merely because it appears in TypeScript or
TypeScript-Go. Tests define compatibility; Rust ownership, data layout, and profiling data
should drive the implementation.
