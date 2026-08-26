# Bootstrap research

Research snapshot: 2026-08-26.

## Reference implementations

- [typescript-go](https://github.com/microsoft/typescript-go) is the compatibility
  reference. Its compiler runner derives tests from upstream TypeScript cases, while
  FourSlash tests cover language-service behavior. `tsrs` should start with compiler
  cases and defer a FourSlash/LSP harness until a project model exists.
- [TypeScript checker notes](https://github.com/microsoft/TypeScript/wiki/Codebase-Compiler-Checker)
  describe the original checker's shared state, lazy type resolution, `TypeFlags`, and
  relation recursion limits. These are behaviors to preserve in tests, not necessarily
  structures to port literally.
- [tsgolint](https://github.com/oxc-project/tsgolint/blob/main/ARCHITECTURE.md) uses the
  native TypeScript-Go AST and checker behind a frontend/backend boundary. Its direct-AST
  advantage cannot be copied when using Oxc, so `tsrs` should avoid building a second AST
  and instead attach type state to stable Oxc node/symbol identities.

## Existing Rust work

- [stc](https://github.com/dudykr/stc) attempted TypeScript compatibility on SWC's AST and
  is now archived. It remains useful design archaeology, especially for type storage and
  conformance work, but should not become a dependency.
- [Ezno](https://github.com/kaleidawave/ezno) is an active Rust checker/compiler with an
  intentionally different, more sound type system and its own parser. It is valuable for
  type-store and effect-model ideas, but it does not target one-to-one `tsc` behavior.

## Bootstrap decisions

1. Pin all Oxc crates to the same release (`0.147.0`) to prevent AST version splits.
2. Keep one crate until profiling or API stability supplies a real boundary.
3. Use Oxc semantic analysis as the binder; do not duplicate scopes and references.
4. Keep checker diagnostics owned and deterministic from day one.
5. Add a benchmark before expanding the type system, and record results only on controlled
   hardware rather than committing misleading one-off numbers.
6. Preserve upstream test provenance in fixtures. Add directive parsing only when a synced
   test actually requires a compiler option or virtual multi-file input.

## Questions to answer with prototypes

- Can Oxc symbol and node IDs remain stable enough for incremental project snapshots, or
  does `tsrs` need its own stable identity layer?
- Should types use a bump arena per program generation or an interner with generation IDs?
- Which relation caches dominate real projects, and what invalidation granularity is useful?
- How much TypeScript diagnostic text must be exact versus structurally equivalent during
  early conformance work?

