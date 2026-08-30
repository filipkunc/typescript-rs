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
Statically named interface members support property reads and explicitly annotated method calls.
The first bounded class milestone supports annotated public instance/static properties and methods,
constructors, `new`, and distinct instance and constructor/static sides.
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
signatures, and member access remain outside that historical property-only milestone; statically
named method signatures and member access are added by the later class prerequisite.

The completed **basic expressions** slice records primitive and currently supported structural
types for simple identifier bindings, whether inferred from an initializer or supplied by an
annotation. A later plain `=` assignment is checked against that stable variable type. Compound
assignments, destructuring assignments, assignment-based narrowing, and use-before-declaration
remain outside this slice. For common incomplete-expression input, the diagnostic adapter also
turns Oxc's generic fatal parse error into the corresponding TypeScript `TS1109` or `TS1128`
diagnostic so partially typed editor input is easier to compare.

The **editor-recovery** integration opts the single-file checker into the pinned Oxc fork's editor
mode. A variable initializer missing after `=` is retained as a zero-width `MissingExpression` at
a safe comma, semicolon, closing-brace, or EOF boundary. The next local slice applies the same node
to a missing simple or compound assignment right-hand side at a source/block semicolon,
closing-brace, or EOF boundary. A third slice recovers missing object-property values at an owned
comma or closing-brace boundary, including inside nested objects, while continuing to check
trustworthy sibling properties. Array assignment RHS and spread operands similarly recover at
owned commas or closing brackets without changing valid sparse-array holes. Calls recover empty
argument slots, assignment RHS values, and spread operands at owned commas/closing parentheses.
The same explicit list contexts recover missing commas and safe missing or nested-mismatched
`}`, `]`, and `)` tokens. Punctuation recovery is recorded as zero-width parser metadata because
there is no expression-shaped AST slot for it.
Incomplete annotations, aliases, union constituents, and object-property types recover as a
zero-width `MissingType`. Safe missing `]`, `)`, and `>` type closers use the same parser metadata
model. The checker leaves a top-level missing type unresolved and gives a directly missing object
property type a cascade-suppressing `any` identity so source-backed sibling errors remain visible.
At source/block initializer and assignment slots, unexpected `:` and `...` text is retained as a
non-empty `MalformedExpression`, distinct from a zero-width missing expression, and receives the
same dependent-diagnostic suppression.
Stage 3 adds parameter slots, parameter/body delimiters, return-expression operands, interface
member types/separators/closers, and missing static or optional member names. A missing member name
retains its real object in `MissingMemberExpression`; an empty parameter is metadata-only and
causes `tsrs` to suppress the incomplete callable signature rather than inventing a binding.
Stage 4 adds a class-member context for missing same-line property separators and an EOF class-body
closer. Both remain punctuation metadata, so the class checker retains real instance/static members.
Stage 5 recovers a call closer before a following declaration and retains a nameless variable
declaration as an empty declarator list, without inventing a symbol. All 25 pinned TypeScript-Go
manifest cases now participate in exact parser/binder parity.
`tsrs` reports the syntax error without assigning a type to the missing node and continues checking
independent code. Other missing expressions and malformed syntax still follow Oxc's existing
behavior until their grammar areas receive focused recovery support.

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
The sidebar displays `fork @<sha>` from the linked checkout and points to the corresponding
`filipkunc/oxc` commit, making the parser implementation under review explicit. The Recovery output
tab uses a parse-only inspection boundary and can compare Normal and Editor mode without sending a
recovered tree through formatting, transforms, minification, code generation, or ESTree
serialization. Named examples cover the Stage 2 expression/list/type edits, Stage 3 function and
interface edits, and the Stage 4 class-member separator/closer path.

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
The adapter converts the checker's UTF-8 byte ranges to the protocol's UTF-16 positions. In the
supported recovery shapes, a missing variable initializer, assignment right-hand side, or object
property/array/call operand, list delimiter, type annotation, supported type closer, or supported
source-backed malformed expression no longer hides independent type diagnostics. The same applies
to the supported function, parameter, interface, class, missing-call-closer, and
missing-declaration-name edits documented in the
[Stage 3 recovery increment](docs/oxc-function-interface-recovery.md),
[class increment](docs/oxc-class-recovery.md), and
[Stage 5 decision](docs/oxc-recovery-upstreaming.md). Completing the edit removes its syntax
diagnostic on the next full-document change.

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

Parser recovery also has an offline [cross-parser manifest](docs/oxc-recovery-manifest.md). Its
checked-in TypeScript-Go probe and 25 narrow edit-state cases pin diagnostics, surviving structure,
declarations, and bindings; all 25 cases participate in exact Oxc parity. Regeneration is
explicit; ordinary Rust tests read the checked JSON and never require Go, a TypeScript-Go checkout,
or network access.

See [Goals.md](Goals.md), [architecture](docs/architecture.md),
[bootstrap research](docs/bootstrap-research.md), and the
[Oxc editor-recovery plan](docs/oxc-editor-recovery-plan.md) for scope and design rationale. The
[recovery status ledger](docs/oxc-editor-recovery-status.md) records implementation evidence and
remaining requirements. The
[first fork change](docs/oxc-first-editor-recovery-change.md) defines the initial missing-expression
AST slice, and the [recovery playground](docs/oxc-recovery-playground.md) defines its visual review
workflow.

## License

Licensed under either the Apache License, Version 2.0 or the MIT License, at your option.
