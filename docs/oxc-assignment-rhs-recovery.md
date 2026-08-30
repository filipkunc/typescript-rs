# Oxc editor recovery: missing assignment right-hand sides

Status: implemented locally as the second narrow recovery slice. It requires the focused Oxc and
`tsrs` gates below before the fork and superproject pins advance.

## Decision

The second fork slice generalizes the first missing-initializer boundary rule just enough to
recover a missing assignment right-hand side in a source or block statement:

```ts
let target: number = 1;
target = ;
const intact: number = "wrong";
```

In editor mode, the assignment retains a zero-width `MissingExpression` at the semicolon and leaves
the semicolon for the expression statement. The following declaration survives parsing and semantic
construction. Normal mode retains Oxc's fatal behavior.

This slice covers simple and compound assignment operators at semicolon, closing-brace, and EOF
boundaries owned by an active source-element or block-statement context. It does not yet recover
sequence expressions, array elements, object property values, arguments, missing delimiters,
source-backed malformed expressions, missing identifiers, or missing types.

## Recovery context and progress contract

Oxc tracks active recovery list contexts separately from ECMAScript grammar flags. Source elements
and block statements add their context for the duration of list parsing and restore the previous
set on exit. Recovery checks run only after an assignment operator has already been consumed and
only when the current token is a boundary owned by an active context:

- source elements own semicolon and EOF boundaries;
- block statements own semicolon, closing-brace, and EOF boundaries.

The assignment parser emits one `TS1109` diagnostic, creates a zero-width `MissingExpression`, and
does not consume the boundary. The enclosing statement or list must then consume the semicolon or
terminate at the closing brace or EOF. This preserves the progress invariant without a global
skip-to-semicolon rule. Valid input does not enter the recovery path, and `ParseMode::Normal`
remains unchanged.

The pinned TypeScript-Go parser provides the behavioral reference: its assignment parser recurses
for the right-hand side, missing primary syntax becomes a zero-width parse-error node, and the
enclosing parsing context owns resynchronization. Oxc keeps its explicit typed `MissingExpression`
representation rather than synthesizing an identifier.

## Required tests and integration gates

The fork slice must prove:

- normal mode remains fatal for the incomplete assignment;
- editor mode recovers deterministically at semicolon, closing-brace, and EOF boundaries;
- simple and compound assignments place exactly one zero-width `MissingExpression` on the RHS;
- visitors and semantic construction remain safe;
- the assignment target reference and later bindings survive without a reference for the missing
  node;
- corresponding valid input is unchanged;
- `tsrs` reports the syntax error and an independent later type error without a dependent cascade;
- the LSP edit sequence removes the recovery diagnostic when the RHS is completed;
- a focused benchmark measures the incomplete assignment path.

Before advancing the pins, run the Oxc generated-code, focused test, full test, parser and semantic
conformance, allocation, benchmark, `just ready`, and CI gates required by the main recovery plan.
Then run the complete `tsrs` formatting, Clippy, tests, rustdoc, benchmark, TypeScript-Go differential
check where available, and playground build/smoke gates.

## Follow-up

The next increments add dedicated contexts and boundary rules for array elements, object property
values, and arguments separately. Each must prevent diagnostics derived only from its
`MissingExpression`, preserve later trustworthy structure, and add a named playground edit state.
