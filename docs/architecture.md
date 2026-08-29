# Bootstrap architecture

## Boundaries

The initial pipeline is deliberately a single crate. Crate boundaries are expensive to
change and the type representation, project model, and incremental strategy are not yet
stable enough to justify them.

- Oxc owns scanning, parsing, its arena-backed AST, scopes, symbols, references, and
  syntax diagnostics.
- `tsrs` owns TypeScript types, assignability, inference, type diagnostics, and eventually
  program/project state.
- The checker borrows Oxc's `Scoping` for one `check_source` run. Declaration `SymbolId`s and
  expression `ReferenceId`s connect syntax to tsrs-owned type and signature tables without
  copying or persisting Oxc semantic data.
- The public boundary returns owned diagnostics. Neither Oxc AST references nor allocator
  lifetimes escape a single-file check.
- The CLI is only a frontend. Checker behavior is tested through the library.

The Oxc source baseline is pinned as the `vendor/oxc` Git submodule so editor-recovery changes to
the AST, parser, generated visitors, and semantic builder can be developed and tested together.
Cargo resolves every Oxc crate from that checkout; normal parser behavior remains the baseline
and the first opt-in recovery slice described in the [recovery plan](oxc-editor-recovery-plan.md)
preserves a missing variable initializer as `MissingExpression`. `check_source` opts into that mode,
keeps the syntax diagnostic, and checks independent declarations in the recovered program while
treating the missing expression as having no trustworthy type. Exact fork and upstream revisions
are recorded in the [compatibility note](oxc-fork.md).

`check_source` currently runs all stages serially for one file. A future `Program` should
own source snapshots and stable file identities, and should schedule independent files in
parallel only after module resolution and dependency tracking exist.

## Near-term type model

Grow the checker around explicit operations instead of mirroring the monolithic upstream
checker:

1. interned primitive and literal type identities with canonical unions (initial support is in place);
2. annotation resolution (initial top-level, non-generic aliases and property-only interfaces are
   in place);
3. assignability with recursion guards and relation caches (initial support is in place);
4. object types and arrays (initial named-property, `T[]`, and built-in `Array<T>` support is in
   place), followed by tuples and index-signature types;
5. expression inference and contextual typing (initial object, array, and union-target support is
   in place), followed by control-flow narrowing;
6. signatures (initial explicitly annotated function declarations and direct calls are in place),
   followed by generics, intersections, and conditional types.

Each operation should be introduced by focused conformance cases and measured against the
existing benchmark before broader upstream suites are enabled.

## Completed checker milestone: property-only interfaces

This checker milestone adds named interfaces as an alternate declaration form for the existing
structural object model. It includes unique top-level interfaces, including named and default
exports, whose bodies contain required or optional statically named property signatures with
explicit annotations. Interface references work wherever the current checker resolves annotations,
including variable declarations and explicitly annotated function parameters and return types.

Interface bodies lower to canonical `TypeKind::Object` identities, so aliases, interfaces, and
object type literals with the same property shape share assignability and contextual typing. The
checker retains borrowed declaration metadata only for the current check so nested missing, excess,
and wrongly typed property diagnostics can preserve interface names and locate the relevant syntax.
This also lets the completed annotated-callable foundation pass and return whole interface values
without adding callable identity to `TypeKind`.

The milestone excludes interface inheritance and declaration merging; recursive and generic
interfaces; method, call, construct, and index signatures; property/member access expressions; and
classes. The intended next sequence is callable interface members with member access, then classes
with separate instance and constructor/static sides.

## Completed checker slice: basic expressions

Simple identifier bindings now populate the existing symbol-to-type table from an explicit
annotation or a supported initializer expression. Plain `=` expressions resolve their left-hand
identifier through Oxc's reference graph and apply the existing assignability and structural
diagnostic operations to the right-hand side. This is declaration-type checking, not flow-sensitive
state: a successful assignment does not replace or narrow the variable's type.

This slice intentionally excludes compound and destructuring assignments, use-before-declaration,
definite-assignment analysis, reassignment of properties or indexed values, and control-flow
narrowing. Fatal parser diagnostics still originate in Oxc. The owned-diagnostic boundary only
normalizes two unambiguous, common incomplete-input shapes to TypeScript-compatible `TS1109` and
`TS1128` codes and messages for the editor loop.

## Current tooling milestone: live single-document diagnostics

The `--lsp` frontend provides the first editor-facing loop without introducing a premature
project model. A Tokio stdio server built on `tower-lsp-server` owns full text snapshots and version
numbers only for documents opened by the client. On open and change it invokes the existing
`check_source` boundary and publishes owned diagnostics; on close it removes the snapshot and
clears diagnostics. Requests are handled sequentially for now so an older check cannot publish
after a newer document version. For the first supported recovery shape, a missing variable
initializer no longer hides later type diagnostics, and completing the initializer removes its
syntax diagnostic on the next full-document check.

The LSP adapter is binary-only frontend code. It translates UTF-8 byte ranges to UTF-16 line and
character positions and does not move Oxc arenas, AST nodes, scopes, or references across checks.
The development VS Code extension is a thin standard LSP client for exercising this same stdio
server. Other clients do not depend on it.

This tooling milestone does not yet define `Program`: it neither scans a workspace nor reads
`tsconfig.json`, watches unopened files, resolves modules, or caches semantic state. Project-wide
diagnostics and language features must wait for stable file identities, configuration ownership,
module resolution, and dependency tracking.

## Next checker milestone: callable interface members and member access

The next checker expansion should add explicitly typed interface methods and property/member access
on the existing object model. This creates a focused semantic path shared by interface values and
the later class instance side. Classes remain the following milestone and should model instance and
constructor/static sides explicitly rather than being introduced as syntax accepted only for the
editor demo.

## Diagnostics and editor use

Diagnostics store UTF-8 byte ranges because that is Oxc's native coordinate system. Structural
checks walk fresh object and array expressions contextually so missing or excess properties and
wrongly typed nested values can point at their relevant syntax. The LSP adapter translates these
ranges to UTF-16 positions at the boundary. Stable codes, owned messages, and
deterministic concise rendering support both editor clients and test baselines without coupling
checker internals to either frontend.

## Test synchronization

Each fixture has a small, explicitly named Rust test so rust-analyzer exposes individual
cases in VS Code's Test Explorer. A guard test detects source fixtures that have not been
registered. Source directives remain comments that can later be interpreted by a harness
options parser. Baselines are sidecars rather than inline assertions, making upstream
source updates easy to diff. Cargo's test-name filter keeps an individual case convenient
to run under a debugger; `BLESS=1` updates only selected baselines.

`cargo test-tsgo` provides a separate differential layer over those same `.ts` sources. It
requires the exact TypeScript-Go revision recorded in `tests/tsgo-reference.txt`, checks the
fixtures under one fixed compiler-options profile, and compares normalized diagnostic codes,
UTF-16 locations, and messages. The `.errors` files remain tsrs regression snapshots; the
differential runner neither consumes nor rewrites them. Keeping these two checks independent
makes a local baseline update unable to conceal drift from the reference implementation.
