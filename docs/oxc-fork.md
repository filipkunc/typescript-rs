# Oxc fork compatibility

`tsrs` carries the Oxc recovery prototype as the `vendor/oxc` Git submodule. Cargo resolves all
direct Oxc dependencies from that checkout; Oxc's own workspace path dependencies keep its AST,
parser, visitors, diagnostics, and semantic crates on one source revision.

## Current baseline

- Published Oxc release previously used by `tsrs`: `0.147.0`
- Upstream release revision (`crates_v0.147.0`): `4e258430cdb290598d9f2aeb2d13be598ec9e8e9`
- Fork: <https://github.com/filipkunc/oxc>
- Recovery branch: `feat/editor-missing-expression`
- Recovery PR: <https://github.com/filipkunc/oxc/pull/1>
- Pinned fork revision: `7cfbb4a3ea39a036e519efe23fc19466ccc1eeaa`
- Fork `main` observed at adoption: `33ac4b0915e66b2908953e85340ad59556449c05`
  (71 commits ahead of the release revision)
- Playground fork: <https://github.com/filipkunc/playground>
- Playground branch: `feat/editor-recovery-playground`
- Playground PR: <https://github.com/filipkunc/playground/pull/1>
- Pinned playground revision: `ebb97febdd6de95acd074bfdf4ca6c0ec4dfc998`
- Required `tsrs` integration changes: Cargo path dependencies, submodule-aware checkout, and no
  checker API changes; the playground opts into `ParseMode::Editor`

The recovery branch starts at the exact release revision rather than bundling the 71 unrelated
post-release commits from fork `main`. It declares the Oxc crates consumed by `tsrs` as version
`0.147.0`. The first fork commit adds opt-in recovery for a variable initializer missing after `=`,
represented by a zero-width, Rust-only `MissingExpression`. Normal parser mode remains the default.

The candidate passed Oxc AST generation, workspace-wide all-target/all-feature compilation and
Clippy with warnings denied, focused parser and semantic tests, affected rustdoc checks, the
production WASM/frontend build, and a headless browser edit-complete-edit smoke test. Oxc's full
all-feature suite reached 145 passing tests before an unrelated codegen snapshot compared the
installed Node `v26.5.1` with its recorded `v26.5.0`; that generated snapshot was not accepted.
The `tsrs` `check_file` benchmark identified the editor-mode check on the normal initializer path;
marking it cold recovered 4.4% in `simple_assignments` and 1.9% in `annotated_callables` relative to
the unhinted implementation.

## Updating the pin

Develop recovery changes on a dedicated branch in the fork. Before recording a newer gitlink:

1. run the focused parser, AST visitor, and semantic tests in `vendor/oxc`;
2. run the complete `tsrs` formatting, linting, test, and rustdoc checks;
3. update this note with the new fork revision and any required `tsrs` API changes;
4. review both the Oxc commit range and the superproject gitlink change.

Fresh checkouts must initialize the submodule with `git submodule update --init --recursive`.
Run `./scripts/oxc-playground setup` once and `./scripts/oxc-playground serve` to inspect the pinned
Oxc revision in Monaco. The frontend's sibling dependency resolves to `vendor/oxc/napi/playground`.
