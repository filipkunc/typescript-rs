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
type and object expression literals with required or optional named properties, `T[]` and
built-in `Array<T>` types with contextually typed array expressions, and non-generic top-level
type aliases and property-only interfaces on explicitly typed variable declarations. Object and
array assignability are structural, including objects and arrays nested in any combination and
inside union targets.
Fresh object literals reject excess properties, structural diagnostics point to nested properties
and array elements, and negative numeric literals are supported. Built-in `Array<T>` is a syntax
rule within that existing subset and does not introduce general generics.

The completed **annotated callable foundations** milestone also supports named function declarations
with explicitly typed simple parameters and return types, parameter and function resolution through
Oxc semantic symbols/references, return-statement checking, and direct calls to named functions.
Calls report TypeScript-compatible argument and exact-arity diagnostics. Return inference, function
and arrow expressions, closures, overloads, optional/default/rest/destructured parameters, generics,
methods, classes, `this`/`super`, and control-flow narrowing remain outside that milestone.

The completed **property-only interfaces** milestone supports unique top-level named interfaces,
including exported declarations, with required or optional statically named properties. Interface
references compose with the existing aliases, object and array shapes, unions, and annotated
function parameters and returns. Interfaces remain structural and reuse canonical object types.
Inheritance, declaration merging, recursion, generics, callable or method signatures, index
signatures, and member access remain outside this milestone.

The completed **basic expressions** slice records primitive and currently supported structural
types for simple identifier bindings, whether inferred from an initializer or supplied by an
annotation. A later plain `=` assignment is checked against that stable variable type. Compound
assignments, destructuring assignments, assignment-based narrowing, and use-before-declaration
remain outside this slice. For common incomplete-expression input, the diagnostic adapter also
turns Oxc's generic fatal parse error into the corresponding TypeScript `TS1109` or `TS1128`
diagnostic so partially typed editor input is easier to compare.

## Try it

Clone with the pinned Oxc fork submodule, or initialize it in an existing checkout:

```console
git clone --recurse-submodules https://github.com/filipkunc/typescript-rs.git
# Existing checkout:
git submodule update --init --recursive
```

```console
cargo run --bin tsrs -- example.ts
cargo test
cargo bench
```

The CLI accepts `.ts`, `.tsx`, `.mts`, `.cts`, and JavaScript extensions understood by
Oxc. It exits with status 1 when diagnostics are emitted.

## Oxc playground

The pinned [Oxc playground](https://github.com/filipkunc/playground) is available at
`vendor/oxc-playground` and builds against this repository's exact `vendor/oxc` revision. It
provides Monaco source editing, AST exploration, diagnostics, scopes, symbols, and control-flow
visualization without using a published Oxc package.

With Node.js 22.18 or newer, Corepack/pnpm, and rustup installed:

```console
# One-time dependency installation and WASM/frontend build
./scripts/oxc-playground setup

# Start the local preview and open the printed URL
./scripts/oxc-playground serve

# Rebuild after changing the Oxc fork or playground frontend
./scripts/oxc-playground rebuild
```

The setup build may take several minutes. Subsequent source editing in Monaco is immediate;
`rebuild` is needed only after changing Rust or frontend implementation. The launcher restores the
generated browser loader after bundling so building the playground does not leave `vendor/oxc`
dirty.

## Language server

`tsrs --lsp` starts a Language Server Protocol server over stdio. The initial LSP milestone
keeps full snapshots of open documents and publishes parser, binder, and checker diagnostics on
open and after every change. Diagnostics are cleared when a document becomes valid or is closed.
The adapter converts the checker's UTF-8 byte ranges to the protocol's UTF-16 positions.

This is intentionally a live single-document view of the checker. It does not yet load
`tsconfig.json`, discover unopened files, resolve imports, or provide hover, completion, navigation,
or other project-aware features. The optional `--stdio` argument is accepted for clients that make
the transport explicit.

To exercise the server in VS Code:

1. Select **Run tsrs LSP in VS Code** in the Run and Debug view.
2. Start debugging. The pre-launch tasks build `target/debug/tsrs` and install the small development
   client under `editors/vscode` from its lockfile.
3. Open a TypeScript file in the Extension Development Host and edit it. Problems produced by this
   server have `tsrs` as their source.

This launch uses an isolated user-data directory and disables installed extensions except the
`tsrs` extension under development. VS Code's built-in TypeScript service remains active as a
reference, so its `ts(...)` diagnostics can be compared directly with `tsrs(...)` while typing.
The isolated profile keeps the experimental TypeScript-Go extension off and suppresses first-run
welcome, release-note, and AI sign-in prompts. It does not change normal VS Code windows or the
project being inspected. The repository client is development scaffolding, not a packaged
Marketplace extension.

Other editors can launch the built binary directly with `target/debug/tsrs --lsp` and stdio
transport. Register it for TypeScript/TSX and, if desired, JavaScript/JSX documents.

## Debugging in VS Code

Install the recommended **rust-analyzer** and **CodeLLDB** extensions when VS Code
prompts you, then press **F5**. The default launch configuration builds `tsrs` with
Cargo and checks the repository's `example.ts` under the debugger.

For a useful first breakpoint, open `src/checker.rs` and place it on the diagnostic
branch inside `Checker::check_variable_declarator`. The example contains a deliberate
type error that reaches this branch. For callable fixtures, use `Checker::check_return_statement`
or `Checker::check_call_expression` instead. For live editor diagnostics, use `Backend::publish`
in `src/lsp.rs`.

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

The same fixture sources can also be checked against a pinned TypeScript-Go build. This is a
differential check: it compares diagnostic codes, UTF-16 line/column locations, and normalized
messages without reading or updating the `.errors` sidecars.

```console
# One-time setup in a TypeScript-Go checkout at tests/tsgo-reference.txt
npm ci
npx hereby local

# Compare all fixtures, or one named fixture
cargo test-tsgo --repo ../typescript-go
cargo test-tsgo --repo ../typescript-go --case primitive_literals

# The checkout can also be supplied through the environment
TSGO_REPO=../typescript-go cargo test-tsgo
```

The runner requires the checkout's exact pinned revision and invokes TypeScript-Go with a fixed
strict, ESNext target/module/library, no-emit profile. A mismatch is evidence to review the fixture
or checker; blessing the local `.errors` baseline does not resolve it. Update
`tests/tsgo-reference.txt` only as an explicit, reviewed reference-version change.

See [Goals.md](Goals.md), [architecture](docs/architecture.md),
[bootstrap research](docs/bootstrap-research.md), and the
[Oxc editor-recovery plan](docs/oxc-editor-recovery-plan.md) for scope and design rationale. The
[first fork change](docs/oxc-first-editor-recovery-change.md) defines the initial missing-expression
AST slice, and the [recovery playground](docs/oxc-recovery-playground.md) defines its visual review
workflow.

## License

Licensed under either the Apache License, Version 2.0 or the MIT License, at your option.
