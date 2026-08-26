# Bootstrap architecture

## Boundaries

The initial pipeline is deliberately a single crate. Crate boundaries are expensive to
change and the type representation, project model, and incremental strategy are not yet
stable enough to justify them.

- Oxc owns scanning, parsing, its arena-backed AST, scopes, symbols, references, and
  syntax diagnostics.
- `tsrs` owns TypeScript types, assignability, inference, type diagnostics, and eventually
  program/project state.
- The public boundary returns owned diagnostics. Neither Oxc AST references nor allocator
  lifetimes escape a single-file check.
- The CLI is only a frontend. Checker behavior is tested through the library.

`check_source` currently runs all stages serially for one file. A future `Program` should
own source snapshots and stable file identities, and should schedule independent files in
parallel only after module resolution and dependency tracking exist.

## Near-term type model

Grow the checker around explicit operations instead of mirroring the monolithic upstream
checker:

1. interned primitive and literal type identities with canonical unions (initial support is in place);
2. annotation resolution (initial top-level, non-generic aliases are in place);
3. assignability with recursion guards and relation caches (initial support is in place);
4. object types and arrays (initial named-property, `T[]`, and built-in `Array<T>` support is in
   place), followed by tuples and index-signature types;
5. expression inference and contextual typing (initial object, array, and union-target support is
   in place), followed by control-flow narrowing;
6. generics, signatures, intersections, and conditional types.

Each operation should be introduced by focused conformance cases and measured against the
existing benchmark before broader upstream suites are enabled.

## Current milestone: standard array type syntax

This checker milestone treats the built-in `Array<T>` type reference as an alternate spelling
of the existing `T[]` type throughout the completed JSON-shaped subset. It includes primitive,
literal-union, object, nested-array, and alias element types, with the same contextual checking and
diagnostic behavior as bracket array syntax.

This milestone is intentionally a built-in syntax rule, not general generic type instantiation. It
does not add user-defined generics, `ReadonlyArray<T>`, tuples, index signatures, array methods, or
standard-library symbol resolution. Completion requires focused registered fixtures and exact
diagnostic parity with the pinned TypeScript-Go revision.

## Diagnostics and editor use

Diagnostics store UTF-8 byte ranges because that is Oxc's native coordinate system. Structural
checks walk fresh object and array expressions contextually so missing or excess properties and
wrongly typed nested values can point at their relevant syntax. A future LSP adapter must
translate these ranges to UTF-16 positions at the boundary. Stable codes, owned messages, and
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
