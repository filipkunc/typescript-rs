# Oxc recovery playground

## Decision

Use Oxc's existing web playground as the human review surface for editor recovery. Extend its
existing Monaco editor and AST explorer rather than building a second UI from scratch.

The playground is a review and exploration tool, not an automated-test replacement. A recovery
slice is not ready for `tsrs` integration until its focused tests and Oxc regression gates pass,
and the behavior can also be inspected in a playground preview.

## Existing pieces to reuse

The Oxc playground is split across two repositories:

- [`oxc/napi/playground`](https://github.com/oxc-project/oxc/tree/main/napi/playground) builds the
  Rust pipeline into a WASM/NAPI binding and already exposes diagnostics, ESTree JSON, Rust debug
  output, scopes, symbols, and control-flow graphs;
- [`oxc-project/playground`](https://github.com/oxc-project/playground) supplies the Vue
  application, Monaco editor, collapsible AST tree, source-to-node hover synchronization, parser
  options, diagnostic markers, split panes, and shareable URL state.

The frontend intentionally links to a sibling `../oxc/napi/playground` checkout. A future fork
checkout at `vendor/oxc-playground` naturally resolves that link to this repository's
`vendor/oxc`. Keep the frontend as its own fork or submodule rather than copying its components
into `tsrs`; that preserves its upstream history and keeps Rust parser work separate from UI work.

## Recovery inspection boundary

Do not route recovered syntax through the playground's existing full `Oxc.run` pipeline. That
pipeline may invoke semantics, linting, formatting, transforms, minification, code generation, and
ESTree serialization. Most batch consumers should continue rejecting recovered trees.

Add a parse-oriented inspection entry point to `napi/playground` which returns owned data suitable
for visualization. Its initial response should contain:

- parse mode and status: clean, recovered, or aborted;
- parser diagnostics and exact UTF-8 source ranges;
- a structural node tree containing node kind, span, recovery state, a short optional label, and
  ordered children;
- a flat recovery-site list containing kind, insertion/skipped range, diagnostic index, and parent
  path;
- surviving top-level declaration names;
- when semantic construction is enabled, surviving symbols/references and any semantic
  diagnostics.

Build the structural tree from Oxc's typed AST and generated visitor events. Do not use ESTree as
the recovery contract: `MissingExpression` is deliberately excluded from ordinary ESTree output,
and future recovered nodes may not have a meaningful ESTree representation. The inspection data
is owned debug data and must not escape into compiler APIs.

The endpoint should be parse-only by default. Semantic inspection is an explicit option and is
enabled only after the recovery node has a tested semantic policy. Formatting, transforms,
minification, linting, and code generation stay disabled for recovered input.

## First UI slice

Add an **Editor recovery** workspace to the existing playground with:

1. Monaco source input on the left, retaining inline parser diagnostics.
2. Normal, Editor, and Compare mode selection; Compare is the useful review default.
3. A compact result header for each mode showing parse status, statement count, recovery-site
   count, diagnostic count, and surviving symbol count.
4. A collapsible recovery tree on the right. Recovered nodes are visually distinct from
   source-backed nodes.
5. A diagnostics/recovery list below the tree with direct navigation to the source location.
6. Shareable URL state containing source, extension, mode, and selected example.

Comparison is structural, not a textual JSON diff. For the first target:

```ts
const broken = ;
const intact: number = "wrong";
```

the header should make the difference immediately visible: normal mode aborts and loses the
program, while editor mode recovers one missing expression and preserves both declarations. The
tree should show `MissingExpression` under `broken` and the intact initializer under `intact`.

Clicking or hovering a source-backed node highlights its Monaco range, and moving the editor caret
expands the narrowest containing tree path. Zero-width recovery nodes require a dedicated Monaco
caret/glyph decoration because the playground's existing half-open range check cannot highlight
an empty span. Multiple missing nodes at one offset must remain separately selectable.

Keep the first UI focused. Scope, symbol, CFG, ESTree, Rust debug text, formatting, and printed
output remain available in the existing playground, but they do not need side-by-side recovery
comparison in the first slice.

## Review workflow

Every recovery grammar slice adds at least one named playground example drawn from its focused
tests. The URL is included in the corresponding fork PR so review can cover both individual edit
states and the full surrounding program.

The reviewer should be able to answer, without reading serialized JSON:

- What construct is missing or malformed?
- Where did recovery resume?
- Which later and enclosing constructs survived?
- Did recovery invent a binding or reference?
- Did one edit produce an obvious diagnostic cascade?
- Does completing the edit remove the recovery node and restore a clean tree?

The UI should support quick manual edit sequences, but important discoveries are promoted to named
automated tests. A shareable playground example is evidence for review, not the lasting regression
test itself.

## Playground validation

The inspection model is tested in Rust independently of WASM. Assert exact status, node hierarchy,
spans, recovery sites, diagnostics, and semantic summaries for the same sources used by parser and
semantic tests.

The frontend adds focused tests for mode/options serialization, zero-width selection, tree
expansion, and comparison summaries. Add one browser smoke test which types a missing initializer,
observes `MissingExpression` and the surviving declaration, completes the initializer, and observes
a clean result. Frontend formatting, type checking, linting, production WASM build, and CI must
pass.

## Repository workflow

Keep two coordinated, independently reviewable changes:

1. the Oxc fork PR adds the AST/parser behavior and the parse-only inspection response;
2. the playground fork PR renders that response and provides a preview URL.

After the playground fork location is chosen, pin it rather than depending on its moving main
branch. Do not advance the `tsrs` Oxc gitlink until the Oxc tests pass and the matching playground
preview has been reviewed. Record both revisions in the compatibility note.
