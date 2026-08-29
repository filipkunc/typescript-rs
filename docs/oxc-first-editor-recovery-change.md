# First Oxc editor-recovery change: missing expressions

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

## Tests and acceptance criteria

Add focused Oxc parser/API fixtures for:

```ts
let value =
const broken = ; const intact = 1;
const first =, second = 2;
function f() { const local = }
```

For each source, assert the exact zero-width missing span and deterministic diagnostic location.
For the corresponding valid source, assert unchanged AST and diagnostics in both modes. Also assert:

- normal mode still reports `panicked = true` and returns an empty program for the incomplete case;
- editor mode reports `panicked = false` and retains the surrounding declaration structure;
- generated `Visit` and `VisitMut` traversals complete without panic;
- semantic construction creates no reference for `MissingExpression` and still binds later and
  enclosing valid declarations;
- every AST span is ordered and within the source range;
- the parser makes progress for comma, semicolon, closing-brace, and EOF boundaries;
- `size_of::<Expression>()` remains unchanged;
- normal-mode parser benchmarks show no meaningful regression.

Run focused parser and semantic tests while iterating, then Oxc's required formatting, generated
file, workspace, conformance, allocation, and benchmark checks. Only after those pass should `tsrs`
advance the submodule gitlink and add integration cases proving that `intact` still receives its
type diagnostic while `broken` produces no dependent type cascade.

## Follow-up

The next recovery slice should generalize the successful boundary/progress rule into parsing
contexts and use `MissingExpression` for assignment right-hand sides, array/object elements, and
arguments. A separate decision is still required for source-backed malformed expressions and for
missing declaration names or type nodes.
