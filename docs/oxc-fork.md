# Oxc fork compatibility

`tsrs` carries the Oxc recovery prototype as the `vendor/oxc` Git submodule. Cargo resolves all
direct Oxc dependencies from that checkout; Oxc's own workspace path dependencies keep its AST,
parser, visitors, diagnostics, and semantic crates on one source revision.

## Current baseline

- Published Oxc release previously used by `tsrs`: `0.147.0`
- Upstream release revision (`crates_v0.147.0`): `4e258430cdb290598d9f2aeb2d13be598ec9e8e9`
- Fork: <https://github.com/filipkunc/oxc>
- Recovery branch: `feat/editor-missing-expression`
- Recovery PR (merged): <https://github.com/filipkunc/oxc/pull/1>
- Pinned fork revision: `e8a03287dbf884ead65393e73488a02b9c099ef2`
- Fork `main` integrated before merge: `33ac4b0915e66b2908953e85340ad59556449c05`
  (71 commits ahead of the release revision)
- Playground fork: <https://github.com/filipkunc/playground>
- Playground branch: `feat/editor-recovery-playground`
- Playground PR (merged): <https://github.com/filipkunc/playground/pull/1>
- Pinned playground revision: `ebb97febdd6de95acd074bfdf4ca6c0ec4dfc998`
- Required `tsrs` integration changes: Cargo path dependencies, submodule-aware checkout, and
  `check_source` opting into `ParseMode::Editor`; the public checker API is unchanged

The recovery work started at the exact release revision and was synchronized with fork `main`
before merge. It declares the Oxc crates consumed by `tsrs` as version `0.147.0`. The first fork
commit adds opt-in recovery for a variable initializer missing after `=`, represented by a
zero-width, Rust-only `MissingExpression`. Normal parser mode remains the Oxc default. `tsrs`
selects editor mode at its single-file parse boundary, runs semantic construction on the recovered
program, and checks independent code only when the recovered node kind has a tested safe policy.

The merged candidate passed Oxc AST generation, workspace-wide all-target compilation, strict
all-target/all-feature Clippy, focused parser and semantic tests, and the all-feature test suite
apart from two environment-dependent snapshot groups: the installed Node `v26.5.1` differs from
the recorded `v26.5.0`, and the non-TTY runner omits expected ANSI styling. No generated snapshots
were accepted. The exact pinned revision also passed the production WASM/frontend build; the
earlier candidate passed a headless browser edit-complete-edit smoke test. The `tsrs` `check_file`
benchmark identified the editor-mode check on the normal initializer path; marking it cold
recovered 4.4% in `simple_assignments` and 1.9% in `annotated_callables` relative to the unhinted
implementation. The final fork-`main` synchronization produced no meaningful regressions in the
same suite; `literal_unions` improved by 4.3%, while the remaining cases stayed within noise.

## Updating the pin

Develop recovery changes on a dedicated branch in the fork. Before recording a newer gitlink:

1. run the focused parser, AST visitor, and semantic tests in `vendor/oxc`;
2. run the complete `tsrs` formatting, linting, test, and rustdoc checks;
3. update this note with the new fork revision and any required `tsrs` API changes;
4. review both the Oxc commit range and the superproject gitlink change.

Fresh checkouts must initialize the submodule with `git submodule update --init --recursive`.
Run `./scripts/oxc-playground setup` once and `./scripts/oxc-playground serve` to inspect the pinned
Oxc revision in Monaco. The frontend's sibling dependency resolves to `vendor/oxc/napi/playground`.
