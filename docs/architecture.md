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

1. interned type identities and primitive/literal types;
2. symbol types and annotation resolution;
3. assignability with recursion guards and relation caches;
4. expression inference and contextual typing;
5. control-flow narrowing;
6. generics, object types, signatures, unions, intersections, and conditional types.

Each operation should be introduced by focused conformance cases and measured against the
existing benchmark before broader upstream suites are enabled.

## Diagnostics and editor use

Diagnostics store UTF-8 byte ranges because that is Oxc's native coordinate system. A
future LSP adapter must translate these to UTF-16 positions at the boundary. Stable codes,
owned messages, and deterministic concise rendering support both editor clients and test
baselines without coupling checker internals to either frontend.

## Test synchronization

Each fixture has a small, explicitly named Rust test so rust-analyzer exposes individual
cases in VS Code's Test Explorer. A guard test detects source fixtures that have not been
registered. Source directives remain comments that can later be interpreted by a harness
options parser. Baselines are sidecars rather than inline assertions, making upstream
source updates easy to diff. Cargo's test-name filter keeps an individual case convenient
to run under a debugger; `BLESS=1` updates only selected baselines.
