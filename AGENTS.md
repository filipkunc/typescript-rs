# Repository working guide

## Start here

Before changing the checker, read:

1. `Goals.md` for the project intent.
2. `docs/architecture.md` for ownership boundaries and the planned type model.
3. `README.md` for the CLI, debugger, tests, and baseline workflow.

Also inspect `git status` and the latest commits. Do not assume a milestone described in
documentation is still the current implementation state.

## Architecture

- Oxc owns parsing, its arena-backed AST, scopes, symbols, references, and syntax-level
  semantic analysis.
- `tsrs` owns TypeScript types, annotation resolution, inference, assignability, and type
  diagnostics.
- Keep Oxc references inside a single `check_source` run. The public boundary returns owned
  diagnostics with UTF-8 byte ranges.
- Keep the project as one crate until the type representation and program model are stable.
- Grow the checker through small explicit operations instead of copying the structure of the
  monolithic TypeScript checker.
- Types are canonical identities interned by `TypeStore`. Put structural assignability in the
  relation layer and protect recursive comparisons with its cache.
- Preserve the allocation-free path for primitive-only checks. Add storage or cloning to hot
  paths only when the modeled type requires it.

## Current milestone: JSON-shaped values

The near-term target is the type-checking subset demonstrated by the
[rendered JSON-validator article](https://filipkunc.com/posts/type-json-validator) and its
[MDX source](https://github.com/filipkunc/FilipKuncCom/blob/main/src/content/posts/type-json-validator/index.mdx):

- object type literals and object literal expressions;
- nested objects and arrays in any combination;
- required and optional object properties;
- primitive, literal, union, and `null` types inside those shapes;
- top-level non-generic type aliases referring to those types;
- structural diagnostics for missing, excess, and wrongly typed properties, including at
  nested locations and inside arrays.

This milestone concerns checking the article's hidden typed assignment, not implementing its
JSON-to-type inferrer, runtime validator generator, or browser UI. Here, JSON "maps" mean
objects with explicitly declared properties. The milestone does not require tuples, recursive
types, generics, index signatures, `Record<K, V>`, the standard-library `Map`, functions,
classes, or control-flow narrowing. Avoid pulling those features in unless they are a small
prerequisite. Once the milestone is complete, update this section rather than leaving stale
status for the next session.

## Conformance-first workflow

- Introduce behavior with focused fixtures under `tests/cases/compiler` before or alongside
  implementation.
- Give every fixture an explicitly named test in `tests/conformance.rs` and register it in
  `REGISTERED_CASES`, so rust-analyzer exposes it in VS Code's Test Explorer.
- Store exact expected diagnostics in the adjacent `.errors` file. A missing or empty sidecar
  means the fixture must pass.
- Keep a fixture's `@reference` comment so its source remains traceable.
- Use `BLESS=1` only after reviewing every diagnostic change; never use it to hide a failure.
- Prefer several narrow cases over one large fixture when they exercise different type-system
  rules.

## Validation

Before handing off a checker change, run:

```console
cargo fmt --all --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo rustdoc --lib -- -D warnings
```

Run `cargo bench --bench check_file` when changing parsing/checking hot paths or type storage.
Keep the existing benchmark names stable so Criterion can compare history, and add a focused
case when a new feature follows a materially different path. Investigate meaningful regressions
instead of accepting them implicitly.

## Human workflow

- Keep the default F5 launch working with the repository's `example.ts`.
- Keep named conformance cases individually runnable and debuggable from Test Explorer.
- Update `README.md` and `docs/architecture.md` when their description of supported behavior or
  debugging locations changes.
- Preserve unrelated working-tree changes. Do not commit or push unless the current conversation
  authorizes it.
