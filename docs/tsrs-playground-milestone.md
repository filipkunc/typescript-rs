# Tooling milestone: tsrs diagnostics in the Oxc playground

## Goal

Add a browser-based checker lab to the pinned Filip Kunc Oxc playground without replacing the
VS Code LSP or moving checker ownership into Oxc. The playground should make supported tsrs
features easy to demonstrate, share, and visually inspect while the LSP remains the source of truth
for real editor lifecycle behavior.

## Ownership and boundary

The browser adapter calls the same `check_source(file_name, source_text)` boundary as the CLI, LSP,
tests, and benchmarks. It returns only owned diagnostics:

- diagnostic code;
- message;
- phase (`parse`, `bind`, or `check`);
- optional UTF-8 byte range.

Oxc continues to own parsing, its arena-backed AST, scopes, symbols, references, and recovery
metadata for one check. No AST, `Scoping`, `SymbolId`, `ReferenceId`, `TypeId`, or `SignatureId`
crosses the JavaScript boundary. The playground invokes its existing Oxc inspection module and the
tsrs checker module independently against the same source text.

The checker remains one Rust crate. A build-only NAPI/WASI frontend crate contains no checker logic;
it depends on the safe tsrs library and projects `check_source` diagnostics into browser DTOs. This
keeps NAPI's generated platform shims outside the checker crate's `unsafe_code = "forbid"` boundary
and introduces no dependency from the Oxc fork back to tsrs. Tokio and `tower-lsp-server` remain
native-only frontend dependencies.

## Browser experience

The pinned playground gains a **tsrs** output tab that:

- displays success or the complete owned diagnostic list;
- shows code, phase, message, and byte range;
- highlights the corresponding Monaco source range on hover or click;
- publishes tsrs Monaco markers alongside the existing Oxc markers;
- offers named examples for JSON shapes, callable expressions, and editor recovery.

The frontend converts UTF-8 byte offsets to Monaco's UTF-16 offsets. The Rust boundary stays in its
native coordinate system so browser presentation cannot change checker semantics.

## Verification

- Rust unit tests prove the browser DTO is an exact projection of `check_source` diagnostics.
- A Node/WASI smoke test invokes the built module on a focused source and checks phase, code,
  message, and byte ranges.
- Frontend unit tests cover UTF-8-to-UTF-16 conversion and diagnostic presentation helpers.
- A headless browser smoke test loads a named tsrs example, selects the tsrs tab, verifies its
  diagnostics, and exercises source highlighting.
- Normal Rust validation, frontend formatting/type/lint/tests/build, and the existing Oxc recovery
  smoke remain required. The WASM artifact size and check latency are recorded so later work can
  detect material regressions.

The first release build produces a 1,252,353-byte WASM module (416,517 bytes through `gzip`). A
local warmed Node/WASI sample of the focused callable example completed 10,000 `checkSource` calls
in 337.8 ms, or 33.8 µs per full parse/bind/check. This is an observational baseline rather than a
cross-machine performance gate; the native Criterion suite remains the stable regression tool.

## Explicit exclusions

- replacing or proxying the LSP;
- multi-file programs, module resolution, `tsconfig.json`, or workspace discovery;
- hover, completion, navigation, code actions, or incremental document state;
- exposing internal types, signatures, scopes, or AST nodes to JavaScript;
- sending a recovered Oxc playground AST into tsrs;
- adding tsrs code to the Oxc fork or publishing a general-purpose browser package.

The first slice is a single-document visualization and demonstration surface only.
