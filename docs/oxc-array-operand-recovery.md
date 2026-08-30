# Oxc missing array operand recovery

This Stage 2 slice adds an explicit array-element recovery context. JavaScript array holes remain
valid elisions and do not create recovery nodes; the malformed states covered here are missing
assignment right-hand sides and missing spread arguments inside an array literal.

## Recovery contract

In `ParseMode::Editor`, the active array-element list owns comma and closing-bracket boundaries.
After an assignment operator or `...`, the parser reports `TS1109`, inserts one zero-width
`MissingExpression`, and leaves the boundary for the array list. Nested arrays and arrays inside
objects retain the same ownership. Normal mode remains fatal, and valid arrays—including sparse
arrays—have identical Normal and Editor ASTs.

Missing commas and missing/mismatched closing brackets are delimiter recovery, not part of this
operand slice. A bare comma is never treated as an error because it denotes an array elision.

## Downstream policy

Semantic construction ignores the missing operand while preserving real assignment references and
later bindings. For a contextually typed array, `tsrs` skips only a recovered direct element or a
spread whose argument is recovered, then continues checking trustworthy sibling elements and later
declarations. It does not infer an element type from the missing node.

Focused parser tests cover assignment and spread positions at comma/closing-bracket boundaries,
nesting, normal-mode isolation, sparse arrays, determinism/progress, and valid-input identity.
Semantic, NAPI inspection, two conformance fixtures, an LSP edit sequence, a benchmark, and a named
playground example cover the integration path.
