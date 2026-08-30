# Oxc fork compatibility

`tsrs` carries the Oxc recovery prototype as the `vendor/oxc` Git submodule. Cargo resolves all
direct Oxc dependencies from that checkout; Oxc's own workspace path dependencies keep its AST,
parser, visitors, diagnostics, and semantic crates on one source revision.

## Current baseline

- Published Oxc release previously used by `tsrs`: `0.147.0`
- Upstream release revision (`crates_v0.147.0`): `4e258430cdb290598d9f2aeb2d13be598ec9e8e9`
- Fork: <https://github.com/filipkunc/oxc>
- Recovery branch: `feat/editor-missing-expression`
- Baseline recovery PR (merged): <https://github.com/filipkunc/oxc/pull/1>
- Complete recovery integration PR: <https://github.com/filipkunc/oxc/pull/2>
- Pinned fork revision: `87e11609811a6c6dc669bf5ea9c2c8d7133a6297`
- Fork `main` integrated before merge: `33ac4b0915e66b2908953e85340ad59556449c05`
  (71 commits ahead of the release revision)
- Playground fork: <https://github.com/filipkunc/playground>
- Playground branch: `feat/editor-recovery-playground`
- Baseline playground PR (merged): <https://github.com/filipkunc/playground/pull/1>
- Complete playground integration PR: <https://github.com/filipkunc/playground/pull/2>
- Pinned playground revision: `dd0d465094c29c1e9fab6362da062f6effbf5fa7`
- Required `tsrs` integration changes: Cargo path dependencies, submodule-aware checkout, and
  `check_source` opting into `ParseMode::Editor`; the public checker API is unchanged

The recovery work started at the exact release revision and was synchronized with fork `main`
before merge. It declares the Oxc crates consumed by `tsrs` as version `0.147.0`. The first fork
commit adds opt-in recovery for a variable initializer missing after `=`, represented by a
zero-width, Rust-only `MissingExpression`. Normal parser mode remains the Oxc default. `tsrs`
selects editor mode at its single-file parse boundary, runs semantic construction on the recovered
program, and checks independent code only when the recovered node kind has a tested safe policy.

The exact pinned candidate passes Oxc AST and linter generation, workspace-wide all-target
compilation, strict all-target/all-feature Clippy, full rustdoc, the unfiltered all-feature test
suite with its recorded Node 26.5.0 runtime and color environment, parser and semantic conformance,
allocation snapshots, and the normal/editor parser benchmark. The reviewed parser conformance
snapshots add only `Opened here` secondary labels for unmatched delimiters. The exact playground
revision passes the production WASM/frontend build, focused frontend tests, type checking, linting,
and the headless browser recovery-and-repair smoke.

## Completed recovery milestone

The pinned revision implements the separately specified
[missing-assignment-RHS](oxc-assignment-rhs-recovery.md) and
[missing-object-property-value](oxc-object-property-value-recovery.md) slices, plus the
[array-operand](oxc-array-operand-recovery.md) and
[call-argument](oxc-call-argument-recovery.md) slices, plus shared
[list-delimiter recovery](oxc-list-delimiter-recovery.md), and the
[type-recovery](oxc-type-recovery.md) slice. They add active
source/block/object-property/array-element/argument/type-argument/parenthesized-type recovery
contexts in the Oxc parser and matching named playground examples. Missing punctuation is carried
as owned zero-width parser recovery metadata; an absent type uses the zero-width `MissingType` AST
variant. The final local Stage 2 increment adds the source-backed `MalformedExpression`
representation and focused source/block initializer and assignment recovery described in
[`oxc-malformed-expression-recovery.md`](oxc-malformed-expression-recovery.md). It also contains the bounded Stage 3 function,
parameter, interface, and member-access increment described in
[`oxc-function-interface-recovery.md`](oxc-function-interface-recovery.md), including
`MissingMemberExpression` and metadata-only empty parameters. Stage 4 adds bounded class-member
recovery, and Stage 5 completes all 25 manifest cases with missing call-closer and nameless variable
declaration recovery. The proposed upstream slices and current decision to retain the fork are in
[`oxc-recovery-upstreaming.md`](oxc-recovery-upstreaming.md). The fork and playground branches are
published in the `filipkunc` repositories at the exact revisions above. Submission to the original
`oxc-project` repositories is intentionally outside the authorized publication scope.

## Updating the pin

Develop recovery changes on a dedicated branch in the fork. Before recording a newer gitlink:

1. run the focused parser, AST visitor, and semantic tests in `vendor/oxc`;
2. run the complete `tsrs` formatting, linting, test, and rustdoc checks;
3. update this note with the new fork revision and any required `tsrs` API changes;
4. review both the Oxc commit range and the superproject gitlink change.

Fresh checkouts must initialize the submodule with `git submodule update --init --recursive`.
Run `./scripts/oxc-playground setup` once and `./scripts/oxc-playground serve` to inspect the pinned
Oxc revision in Monaco. The frontend's sibling dependency resolves to `vendor/oxc/napi/playground`.
