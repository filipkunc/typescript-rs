# TypeScript in Rust

[![CI](https://github.com/filipkunc/typescript-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/filipkunc/typescript-rs/actions/workflows/ci.yml)

`typescript-rs` is an experiment in implementing TypeScript's type checker in Rust. The
command-line executable remains the shorter `tsrs`. The project uses
[Oxc](https://oxc.rs/) for parsing and syntax-level semantic analysis, leaving this
project focused on type relationships, inference, project construction, and diagnostics.

This is an independent experimental project and is not affiliated with or endorsed by
Microsoft.

This repository is in its bootstrap phase. Today it provides a thin working pipeline:

```text
TypeScript source -> Oxc parser -> Oxc binder -> tsrs checker -> owned diagnostics
```

The checker currently recognizes primitive and literal types, canonicalized unions, object
type and object expression literals with required or optional named properties, `T[]` types and
contextually typed array expressions, and non-generic top-level type aliases on explicitly typed
variable declarations. Object and array assignability are structural, including objects and
arrays nested in any combination and inside union targets. Fresh object literals reject excess
properties, structural diagnostics point to nested properties and array elements, and negative
numeric literals are supported. `Array<T>` remains follow-up work beyond the completed
JSON-shaped values milestone.

## Try it

```console
cargo run -- example.ts
cargo test
cargo bench
```

The CLI accepts `.ts`, `.tsx`, `.mts`, `.cts`, and JavaScript extensions understood by
Oxc. It exits with status 1 when diagnostics are emitted.

## Debugging in VS Code

Install the recommended **rust-analyzer** and **CodeLLDB** extensions when VS Code
prompts you, then press **F5**. The default launch configuration builds `tsrs` with
Cargo and checks the repository's `example.ts` under the debugger.

For a useful first breakpoint, open `src/checker.rs` and place it on the diagnostic
branch inside `Checker::check_variable_declarator`. The example contains a deliberate
type error that reaches this branch.

The Run and Debug selector also contains **Debug conformance case**. It prompts for a
named fixture test and launches the Rust test executable under CodeLLDB.

The Testing sidebar shows unit and conformance tests through rust-analyzer's built-in
Test Explorer integration. Each TypeScript fixture has an explicitly named `#[test]`, so
it can be run or debugged independently from the tree. No separate test-adapter extension
is required.

## Conformance workflow

Compiler fixtures live in `tests/cases/compiler`. Each `.ts` file has a named test in
`tests/conformance.rs`; its optional `.errors` sidecar is the exact expected output. A
missing or empty sidecar means the file should pass. A guard test fails if a fixture has
not been registered, keeping the Test Explorer tree complete.

```console
# Run one case while debugging
cargo test --test conformance primitive_literals -- --nocapture

# Accept current output as the baseline
BLESS=1 cargo test --test conformance primitive_literals
```

Fixtures may retain TypeScript test directives as comments. Add an `@reference` comment
with the upstream path or a bootstrap identifier so a case remains traceable during
synchronization.

See [Goals.md](Goals.md), [architecture](docs/architecture.md), and
[bootstrap research](docs/bootstrap-research.md) for scope and design rationale.

## License

Licensed under either the Apache License, Version 2.0 or the MIT License, at your option.
