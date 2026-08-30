# Oxc editor-recovery implementation status

This ledger maps the requirements in
[`oxc-editor-recovery-plan.md`](oxc-editor-recovery-plan.md) to authoritative repository evidence.
It records the current working tree, not merely the last pinned revisions. A checked item means the
named implementation and focused evidence exist; it does not substitute for the broader release
gates listed at the end.

## Stage 0: baseline and research

- [x] Pin the Oxc fork, playground fork, and TypeScript-Go reference revisions.
- [x] Document Oxc's fatal path and the TypeScript-Go recovery/list-context model.
- [x] Establish the submodule-based Oxc and playground workflow.
- [x] Define and generate the cross-parser recovery manifest and initial reference corpus.
- [x] Check in a pinned TypeScript-Go manifest probe without creating a network-dependent test.

Evidence: [`oxc-fork.md`](oxc-fork.md),
[`oxc-first-editor-recovery-change.md`](oxc-first-editor-recovery-change.md), and
[`oxc-recovery-playground.md`](oxc-recovery-playground.md). The manifest schema, 27-case corpus,
checked-in probe, explicit generator, and offline Oxc parity tests are
documented in
[`oxc-recovery-manifest.md`](oxc-recovery-manifest.md).

## Stage 1: recovery infrastructure

- [x] Provide default-normal, opt-in `ParseMode::Editor`.
- [x] Add the explicit zero-width `MissingExpression` representation with generated visitor and
  semantic support.
- [x] Preserve a recovered program for the implemented initializer, assignment, and object-value
  slices.
- [x] Add recovery-context tracking and progress invariants for every implemented source, block,
  variable-declaration, object, array, argument, parameter, type, interface, and class list owner.
- [x] Add span, visitor, semantic-safety, determinism, and valid-input tests, including the Stage 5
  deletion matrix over twelve independently derived edit states.
- [x] Expose an owned parse-only recovery inspection response with mode/status, structural tree,
  recovery sites, diagnostics, declaration names, and optional semantic summaries.
- [x] Implement Normal/Editor/Compare recovery UI, structural summaries, recovery-site navigation,
  caret-driven tree expansion, and zero-width selection tests. Focused frontend tests and an
  automated headless-Chrome edit smoke cover the recovered-to-clean transition.

## Stage 2: current checker grammar

- [x] Recover missing variable initializers at comma, semicolon, closing-brace, and EOF boundaries.
- [x] Recover a missing initializer before an unambiguous following `const` or `var` by querying
  active variable-declaration and source/block contexts; preserve the next statement with or
  without a newline.
- [x] Recover missing assignment right-hand sides for the bounded source/block semicolon,
  closing-brace, and EOF contexts.
- [x] Recover the bounded array-element operands and delimiters while preserving ordinary elisions.
- [x] Recover the bounded object-property values and safe delimiters used by the checker milestone.
- [x] Recover bounded argument operands and call delimiters, including a missing closer owned by a
  following declaration.
- [x] Recover incomplete type annotations and relevant type/list delimiters. The local
  slice adds `MissingType` for annotation, alias, union-constituent, and object-property positions,
  plus safe missing `]`, `)`, and `>` recovery. Focused parser, semantic, inspection, conformance,
  LSP, benchmark, documentation, and named-playground-example coverage exists; the pinned
  differential matches all parser diagnostics exactly. Full gates pass; the exact pin updates were
  completed with the final Stage 5 publication below.
- [x] Add source-backed malformed-expression representation and cascade policy where
  required. The local slice adds non-empty `MalformedExpression` nodes for `:` and `...` in
  source/block initializer and assignment slots, plus checker suppression and focused parser,
  semantic, inspection, conformance, LSP, benchmark, documentation, and playground coverage. Full
  gates pass; the exact pin updates were completed with the final Stage 5 publication below.
- [x] Preserve a statement-position variable declaration missing its name without inventing a
  declarator or binding; exact TS1134 diagnostics and the following statement survive.

Evidence for the local assignment slice:
[`oxc-assignment-rhs-recovery.md`](oxc-assignment-rhs-recovery.md), Oxc's
`crates/oxc_parser/tests/editor_recovery.rs` and
`crates/oxc_semantic/tests/integration/recovery.rs`, and the named `tsrs` conformance/LSP tests.
Evidence for the object-value slice: [`oxc-object-property-value-recovery.md`](oxc-object-property-value-recovery.md)
and the same parser, semantic, inspection, conformance, LSP, benchmark, and playground surfaces.
The pinned TypeScript-Go diagnostic differential agrees on every recovery `TS1109` code and
location, but intentionally differs on independent `TS2322` diagnostics: TypeScript-Go stops its
checker after the parse error, while the editor-mode `tsrs` contract continues over trustworthy
recovered syntax. The recovery manifest must record parser parity separately from this explicit
checker-policy divergence.
Evidence for the array-operand slice: [`oxc-array-operand-recovery.md`](oxc-array-operand-recovery.md)
and its parser, semantic, inspection, conformance, LSP, benchmark, and playground tests.
Evidence for the call-argument slice: [`oxc-call-argument-recovery.md`](oxc-call-argument-recovery.md)
and the corresponding evidence surfaces.
Evidence for shared object/array/call delimiter recovery:
[`oxc-list-delimiter-recovery.md`](oxc-list-delimiter-recovery.md), the named missing-comma and
missing-closer conformance cases, the delimiter LSP edit sequence and benchmark, and the focused
parser, semantic, inspection, and playground tests. The pinned TypeScript-Go differential matches
all six delimiter `TS1005` codes, messages, and insertion locations. As with the expression slices,
the only fixture-level difference is intentional continued `tsrs` checking after parse recovery.
Affected Oxc formatting, strict clippy, parser/semantic/NAPI tests, AST generation, and allocation
snapshots pass. Full `tsrs` gates, WASM/frontend formatting, unit/type/lint/production builds, and
the headless-Chrome edit/navigation smoke also pass. Restoring the list loop's prior always-inline
contract removed the initially observed valid-list benchmark regression; the focused rerun returned
`small_file` to its prior range and showed no further change in `object_shapes`.
Evidence for missing types and type closers: [`oxc-type-recovery.md`](oxc-type-recovery.md), the
named missing-type conformance cases, the type-recovery LSP edit sequence and benchmark, and the
focused parser, semantic, inspection, and playground tests.
Evidence for source-backed malformed expressions:
[`oxc-malformed-expression-recovery.md`](oxc-malformed-expression-recovery.md), the named
conformance case, LSP edit sequence, benchmark, and focused Oxc/inspection/playground tests. The
pinned differential matches both parser diagnostics exactly; only the intentional continued-check
`TS2322` differs.
The first malformed-expression benchmark run caught a wildcard-arm recovery check on every primary
expression; restricting it to only `:` and `...` restored `small_file` to about 4.68 microseconds
and improved the focused callable, assignment, and call-recovery reruns by 3–6%. Oxc formatting,
generation, strict affected-package Clippy, focused tests, allocation snapshots, full `tsrs`
validation, WASM/frontend gates, and the headless browser smoke all pass for the complete local
Stage 2 set.

## Stage 3: functions and interfaces

- [x] Recover parameter slots, parameter delimiters, and parameter types without inventing a
  binding for an empty slot.
- [x] Recover function-body closers, return-expression operands, and ambient return types.
- [x] Recover interface member types, separators, and closers, plus static and optional
  member-access names.
- [x] Harden semantic construction and `tsrs` cascade suppression for every new recovered shape.

Evidence: [`oxc-function-interface-recovery.md`](oxc-function-interface-recovery.md), the named
parser, semantic, inspection, conformance, and LSP tests; the 21-case implemented recovery-manifest
parity set; `editor_recovery_function_interface_edits`; and the `function-interface-edits`
playground example. Oxc generation, formatting, strict affected-package Clippy, parser/semantic/
inspection tests, allocation snapshots, full `tsrs` validation, Criterion, the fresh WASM build,
frontend formatting/unit/type/lint/production-build gates, and the headless browser smoke pass.
The exact fork and playground pin updates were completed with the final Stage 5 publication below.

## Stage 4: classes

- [x] Implement the bounded class checker milestone with explicit instance and constructor/static
  sides, shared callable-member signatures, `new`, and static member access.
- [x] Recover missing class-member separators and EOF class-body closers without moving class
  semantics into Oxc.

Evidence: [`class-checker-milestone.md`](class-checker-milestone.md),
[`oxc-class-recovery.md`](oxc-class-recovery.md), the named member/class/recovery conformance and
LSP cases, pinned TypeScript-Go differential checks, the then-current 23-case recovery parity set,
focused Oxc parser/semantic/inspection tests, `class_sides`, `editor_recovery_class_member`, and the named
class playground example. Oxc formatting, strict Clippy, parser/semantic/inspection tests,
allocation snapshots, full `tsrs` validation and rustdoc, the pinned checker differential,
Criterion, fresh WASM/frontend gates, and the headless browser smoke pass. The performance pass
keeps `TypeKind` compact, moves class/member maps behind one lazy state allocation, and consolidates
top-level declaration discovery; repeated focused runs leave primitive/object/array paths unchanged
within Criterion's noise threshold. The exact fork/playground pin updates were completed with the
final Stage 5 publication below.

## Stage 5: broader parity and upstreaming

- [x] Grow the edit-state corpus with the final TypeScript-Go cases, a twelve-case deletion matrix,
  and a four-version real LSP edit sequence.
- [x] Keep mutation reductions as permanent named parser, semantic, inspection, conformance, and LSP
  tests.
- [x] Prepare isolated recovery primitives and grammar fixes for upstream review.
- [x] Publish the tested integration commits and pull requests in the `filipkunc` Oxc and
  playground forks.
- [ ] Publish and submit the isolated recovery primitives and grammar fixes upstream.
- [x] Benchmark and decide whether fine-grained incremental parsing is needed; it is deferred until
  the `Program` model and realistic editor traces identify a material bottleneck.
- [x] Reevaluate the fork at the local Stage 5 checkpoint; it remains necessary today.
- [ ] Reevaluate the fork after upstream adoption and retire it only when upstream supplies every
  recovery representation and semantic guarantee on which `tsrs` depends.

Evidence: [`oxc-recovery-upstreaming.md`](oxc-recovery-upstreaming.md), exact 27/27 offline manifest
parity, `editor_mode_deletion_mutation_matrix_is_deterministic_and_bounded`,
`publishes_deterministic_diagnostics_across_deletion_and_repair_sequence`, the two new conformance
fixtures, focused semantic and NAPI inspection tests, the `stage-five-deletion-recovery` playground
example, and the `editor_recovery_missing_call_closer` and
`editor_recovery_missing_declaration_name` benchmarks. The fork also has a dedicated
`parser_editor_recovery` Criterion group comparing normal-valid, editor-valid, and representative
editor-incomplete parsing. The authorized fork integration and exact pin work is complete. The
original Stage 5 remains open only for submission to the original projects and post-adoption
reevaluation, both outside the authorized publication scope.

The current integration is published as
`filipkunc/oxc@a2d89696356a5893bd2c0c49ad938088fad1819e` and the UI as
`filipkunc/playground@fe4bb0e161948fa15df6e34ac1df25a51ab06f26`. They build on the recovery
baselines merged in each fork's PR 2. Original `oxc-project` submissions are outside the authorized
publication scope, so the corresponding original-plan items remain open.

## Release and completion gates

The full plan is not complete until every implemented slice has its named playground example and
the recovery manifest agrees with the pinned TypeScript-Go behavior, and until all required Oxc,
playground, and `tsrs` formatting, generated-code, lint, unit/integration, conformance, allocation,
benchmark, rustdoc, production-build, browser-smoke, and CI gates pass. Exact fork and playground
revisions must then replace any pending-local status in [`oxc-fork.md`](oxc-fork.md).

All runnable local gates pass for the final working tree: root formatting, strict Clippy,
all-target/all-feature tests, rustdoc, the complete Criterion suite and focused reruns; Oxc AST and
linter generation, formatting, workspace check, full strict Clippy, full documentation, 68 parser
recovery tests, 19 semantic recovery tests, 21 NAPI inspection tests, and allocation snapshots; a
fresh WASM build; frontend formatting, four unit tests, type-check/lint, production build; and the
headless browser smoke. The full Oxc Rust suite passes unfiltered with `NO_COLOR` unset and its
stacktrace snapshot's recorded Node 26.5.0 runtime.

Pinned Test262, Babel, and TypeScript parser and semantic conformance pass. The parser run produced
two reviewed snapshot updates: eleven Babel cases and five TypeScript cases now show an additional
`Opened here` label for the unmatched delimiter; case classification and pass counts are
unchanged. A repeat parser run left both snapshot hashes unchanged. The fork-level parser benchmark
measures about 1.55 microseconds for normal-valid input, 1.62 microseconds for the same input in
editor mode, and 1.85 microseconds for representative incomplete editor input on this machine.
Root Criterion reports valid paths unchanged or within noise and the reordered cold guards return
the directly affected recovery paths to no change/within noise. `typos` and dependency-shear checks
pass. No `.actual` or `.snap.new` files exist and all three repository diffs pass
`git diff --check`.

After the reviewed Oxc integration commit, the literal `just ready` wrapper passes from its clean
working tree with the recorded Node and color environment.

Hosted fork/playground CI has been dispatched for the published integration commits. The exact
gitlinks and compatibility revisions are part of the `typescript-rs` integration publication.
Original-project submission and the post-adoption fork reevaluation remain open but are outside the
authorized publication scope. GitHub CLI authentication through the host keyring has been verified
with the required repository and workflow scopes; sandboxed commands may not be able to read that
keyring without escalation.
