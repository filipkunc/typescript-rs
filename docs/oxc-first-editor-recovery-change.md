# First Oxc editor-recovery change: missing expressions

Status: implemented and merged in the pinned Oxc and playground forks. The `tsrs` integration gate
is also complete: `check_source` opts into editor mode, a focused conformance case proves that a
later declaration is still checked without a cascade from `MissingExpression`, and the LSP test
completes the initializer and observes the recovery diagnostic disappear.

## Decision

The first fork change should add an explicit zero-width `MissingExpression` AST node and emit it
for a missing variable initializer in opt-in editor mode. The first end-to-end target is:

```ts
const broken = ;
const intact: number = "wrong";
```

The recovered program must retain both variable statements. `broken` must have a
`MissingExpression` initializer at the semicolon, and semantic construction must still bind
`intact`. Normal parser mode must retain Oxc's current fatal result for the same source.

This is deliberately narrower than general expression or list recovery. It proves that a recovery
node can move through Oxc's generated AST machinery and semantic builder safely before recovery
contexts are introduced across the grammar.

## Evidence from the pinned implementations

At the pinned Oxc `0.147.0` revision, `set_fatal_error` advances the lexer to EOF and generic parser
error paths return a `Dummy` value of the required type. `ParserImpl::parse` then replaces the
partially built program with `Program::dummy`. For a missing initializer, the dummy expression is
currently a `NullLiteral`, so preserving that partial program would incorrectly turn absent syntax
into source-level `null`.

At the pinned TypeScript-Go reference revision, `parseInitializer` calls the normal assignment
expression parser after `=`. A missing primary expression becomes a zero-width identifier with an
empty name and the parse-error flag, without consuming the boundary token. The surrounding
variable-declaration and statement lists then consume or terminate at that boundary and continue.

Oxc should preserve that observable recovery behavior without copying the representation. An
explicit missing expression is safer in Oxc's typed AST because visitors and semantic analysis
cannot mistake it for a real identifier reference or literal.

## AST contract

Add `MissingExpression` to `oxc_ast` with only the standard `node_id` and `span` fields, and add it
as an owned variant of `Expression` using the next free discriminant below the inherited member
expression range.

The node contract is:

- its span is always zero-width at the insertion point;
- it represents expected-but-absent syntax only, never skipped source text;
- it has no name, value, symbol, reference, or child nodes;
- generated immutable and mutable visitors enter and leave it normally;
- semantic construction assigns its node identity but creates no binding or reference;
- it is excluded from ESTree serialization because recovered ASTs are not valid transform,
  formatter, minifier, or code-generation inputs;
- adding it must not change the 16-byte size of `Expression`.

Source-backed malformed expressions should eventually use a separate error representation. Keeping
missing and malformed syntax distinct makes spans and cascade suppression unambiguous.

Run Oxc's AST generator (`just ast`) after editing the source definitions. Generated files must not
be hand-edited. Audit inherited expression unions and handwritten exhaustive matches across the
workspace; consumers that cannot accept recovered syntax should reject the new variant explicitly
rather than print or transform invented code.

## Parser API and behavior

Add a public `ParseMode` with `Normal` as its default and `Editor` as the opt-in alternative, exposed
through `ParseOptions`. Normal mode must keep the current fast-forward-to-EOF and dummy-program
behavior byte-for-byte for this slice.

In `parse_variable_declarator`, after consuming `=`, editor mode may synthesize a
`MissingExpression` only when the current token is a boundary owned by the surrounding grammar:

- comma, consumed by the variable-declaration list;
- semicolon, consumed by statement termination;
- closing brace, consumed by the enclosing block;
- EOF, which terminates the enclosing source or block list.

The missing node does not consume the boundary token. The parser emits one expression-expected
diagnostic at that location and returns the node as the declarator's initializer. Because the
surrounding construct owns the next action, every recovery path either consumes input or exits a
known list; the new path cannot spin.

The normal valid-source path should check the token boundary before reading `ParseMode`, keeping the
extra mode branch on an already erroneous path. Parser status does not need another public field in
this first change: clean, recovered, and aborted parses remain distinguishable through diagnostics,
the explicit recovery node, and the existing `panicked` flag. A broader status API can be added when
recovery is no longer represented by a single node kind.

## First fork PR scope

The first fork PR should contain four reviewable commits:

1. Add `MissingExpression`, regenerate AST builders/derives/visitors, and make the Oxc workspace
   compile with explicit unsupported-consumer handling.
2. Add the default-normal `ParseMode` API without changing parser behavior.
3. Emit `MissingExpression` for missing variable initializers at the four safe boundaries in editor
   mode, with a single nonfatal diagnostic.
4. Add semantic and invariant tests proving visitor safety, binding preservation, span validity,
   determinism, and normal-mode compatibility.

Do not add general recovery contexts, assignment-RHS recovery, missing identifiers or types,
argument/parameter recovery, skipped-token nodes, diagnostic-code normalization in `tsrs`, or
support for recovered nodes in Oxc's batch consumers in this PR.

## Playground review surface

The first fork PR should expose the recovered program through the parse-only inspection response
defined in [`oxc-recovery-playground.md`](oxc-recovery-playground.md). A coordinated playground PR
must provide a preview with normal/editor comparison before the new Oxc revision is adopted by
`tsrs`.

For the first target, the recovery tree must show `MissingExpression` as the initializer of
`broken`, retain `intact`, render the zero-width insertion point in Monaco, and clear the recovery
node when an initializer is typed. This inspection response is separate from ESTree and does not
authorize formatter, transform, minifier, linter, or code-generation support for recovered nodes.

## Tests and acceptance criteria

The test layers are additive. The existing Oxc corpus is the regression net for normal parsing,
but it cannot prove an opt-in editor-mode contract that the corpus does not invoke. Conversely,
focused unit tests make the new behavior reviewable and debuggable, but cannot replace Oxc's
Test262, Babel, and TypeScript coverage. The fork change does not advance until both layers pass.

### Focused Rust tests

Add individually runnable parser API tests for:

```ts
let value =
const broken = ; const intact = 1;
const first =, second = 2;
function f() { const local = }
```

These tests belong at the public `Parser`/`ParseOptions` boundary because they exercise behavior
that Oxc's normal-mode conformance runner cannot select. Give each boundary and behavior an
explicit test name rather than hiding the cases in one snapshot. For every source, parse in both
`Normal` and `Editor` modes and assert the observable result directly:

- normal mode still reports `panicked = true` and returns an empty program for the incomplete case;
- editor mode reports `panicked = false` and retains the surrounding declaration structure;
- the initializer is exactly one `MissingExpression` with the expected zero-width insertion span;
- the boundary token is left for its owning declaration, statement, or block parser;
- exactly one parser diagnostic is emitted at the expected location;
- a second parse produces the same AST shape, spans, status, and diagnostics;
- corresponding valid input has the same AST and diagnostics in both modes.

Add AST/visitor unit tests which construct the node without involving parser recovery and assert:

- generated `Visit` and `VisitMut` traversals complete without panic;
- both visitors enter and leave the missing node exactly once;
- its span is empty and `size_of::<Expression>()` remains 16 bytes.

Add semantic integration tests which parse a recovered program and assert:

- semantic construction creates no reference for `MissingExpression` and still binds later and
  enclosing valid declarations;
- every AST span is ordered and within the source range;
- semantic traversal and syntax checking complete without panic.

Finally, add a table-driven progress test for comma, semicolon, closing-brace, and EOF boundaries.
Include repeated adjacent failures so the test proves that every recovery step either consumes a
token or returns to an enclosing list. A timeout alone is not an adequate progress assertion.

Prefer structural assertions for the small contract above. Snapshots may supplement diagnostics,
but must not be the only proof of node kind, span, parser status, or surviving bindings.

### Existing Oxc regression suites

Run the following gates from the Oxc checkout for every candidate fork commit:

1. Run `just ast` and review the generated AST, builder, derive, and visitor diff; generated files
   must be reproducible and must not be hand-edited.
2. Run focused crate tests while iterating, then `just test` for the full Rust test suite.
3. Run `cargo coverage -- parser` so Test262, Babel, and TypeScript parser conformance all retain
   their existing normal-mode results. Any snapshot change must be explained; the expected result
   for this opt-in first slice is no normal-mode change.
4. Run the semantic conformance and integration tests after adding the new semantic case.
5. Run `just allocs`, normal-mode parser benchmarks, and editor-mode benchmarks on valid and
   representative incomplete input. Normal valid parsing must show no meaningful regression.
6. Run `just ready` as the final Oxc pre-push gate and require the fork's CI to pass.

Do not add duplicate normal-mode syntax fixtures when Test262, Babel, or TypeScript already covers
the grammar. Add a local Oxc coverage fixture only for a syntax regression absent from those
suites. The new Rust API tests remain required because external suites do not enable editor mode
or inspect its recovery representation.

Mutation coverage follows the deterministic first slice: delete or replace tokens around `=`,
commas, semicolons, braces, and EOF; assert no panic and the progress invariant; reduce every
failure into a named permanent test. This can start as a bounded table-driven test and grow into a
fuzz target once the recovery contexts expand.

### `tsrs` integration gate

After the Oxc gates passed, `tsrs` advanced the submodule gitlink and added the named
`recovered_missing_initializer` conformance fixture with its exact `.errors` sidecar. The fixture
proves that `intact` still receives its type diagnostic while annotated `broken` produces no
dependent type cascade. The LSP edit-sequence test observes the incomplete state and then completion
of the initializer. The existing `tsrs` conformance suite and the repository's full formatting,
lint, test, rustdoc, and relevant benchmark gates remain required for every later pin.

## Follow-up

The next recovery slice should generalize the successful boundary/progress rule into parsing
contexts and use `MissingExpression` for assignment right-hand sides, array/object elements, and
arguments. A separate decision is still required for source-backed malformed expressions and for
missing declaration names or type nodes.
