# Oxc source-backed malformed-expression recovery

This final Stage 2 grammar increment distinguishes an expected-but-absent expression from source
text that is present but cannot form an expression. It is deliberately narrow: it covers `:` and
`...` in initializer or assignment right-hand-side slots owned by source or block statement
contexts.

## Representation and parser policy

In opt-in `ParseMode::Editor`, `MalformedExpression` retains the unexpected token's full non-empty
span and consumes exactly that token. It reports `TS1109 Expression expected.` and lets the owning
statement list resume at the following semicolon, closing brace, or EOF. The node has no children,
binding, reference, or inferred value. Generated visitors see it, semantic construction treats it
as inert, and it is excluded from ESTree and batch-oriented output paths.

`MissingExpression` remains exclusively zero-width. Keeping the two nodes distinct makes recovery
inspection and checker suppression precise: source-backed malformed text is navigable as a range,
while absent text remains an insertion caret.

This increment does not reinterpret binary or conditional operators, invalid lexer tokens, or
malformed tokens inside object, array, or argument lists. Those list contexts retain their existing
normal behavior unless they receive a separately referenced context-specific recovery rule. Normal
parse mode is unchanged.

## Checker policy

`tsrs` assigns no type to `MalformedExpression`, so an annotated initializer or assignment does not
produce a dependent assignability diagnostic. Independent declarations continue to bind and check.
The same recovery predicate is used by existing structural and argument paths so future explicitly
supported nested malformed nodes cannot suppress diagnostics from trustworthy siblings.

## Evidence

Focused parser tests assert normal-mode aborts, exact source-backed spans, TypeScript-compatible
`TS1109`, cursor progress, deterministic ASTs and diagnostics, generated visitor events, and the
unchanged 16-byte `Expression` layout. Semantic and playground-inspection tests prove the nodes are
inert, retain their source ranges, and preserve later bindings and assignment references.

The `recovered_malformed_expressions` conformance fixture and LSP edit sequence cover both an
assignment RHS and a variable initializer, with an independent `TS2322` surviving until the edit is
completed. The pinned TypeScript-Go differential matches both parser diagnostics exactly in code,
message, and location; its absent `TS2322` is the existing intentional continued-checking policy.
The `editor_recovery_malformed_expression` benchmark and `malformed-expression` playground example
exercise this path. The first benchmark run exposed a 4–6% valid-input regression because the
recovery guard was attached to the wildcard primary-expression arm. Restricting the guard to only
the `:` and `...` token arms restored `small_file` to about 4.68 microseconds and improved the
focused callable, assignment, and call-recovery reruns by 3–6% relative to the regressed samples.
Oxc formatting, generation, strict affected-package Clippy, focused tests, allocation snapshots,
the full `tsrs` gates, WASM/frontend checks, and browser smoke pass. Exact fork/playground pin
updates remain pending.
