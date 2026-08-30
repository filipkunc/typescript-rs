# Oxc missing call-argument recovery

This Stage 2 slice adds an explicit argument-list recovery context for ordinary calls and `new`
expressions. It covers missing assignment right-hand sides, missing spread operands, and an empty
argument between commas while leaving call-delimiter recovery for a separate increment.

## Recovery contract

In `ParseMode::Editor`, an active argument list owns comma and closing-parenthesis boundaries.
After an assignment operator or `...`, the parser reports `TS1109` and inserts a zero-width
`MissingExpression`. When an argument element itself is absent before a comma, it reports the
TypeScript-compatible `TS1135` and inserts the same node as an `Argument`. Boundaries remain
unconsumed for the delimited-list parser, trailing commas remain valid, and later arguments and
statements survive. Normal mode stays fatal and valid calls have identical ASTs in both modes.

Missing commas and missing/mismatched closing parentheses remain part of delimiter recovery.

## Downstream policy

Semantic construction keeps real callee and assignment-target references but invents none for a
missing argument. `tsrs` suppresses checking only for the recovered argument or recovered spread
operand and continues checking later arguments against their corresponding parameters. Exact arity
still counts the recovered syntactic slot, avoiding a dependent arity diagnostic.

Focused parser tests cover all three missing forms at comma/closing-parenthesis boundaries,
normal-mode isolation, determinism/progress, trailing commas, and valid-input identity. Semantic,
NAPI inspection, three conformance fixtures, an LSP edit sequence, a benchmark, and a named
playground example cover the integration path.
