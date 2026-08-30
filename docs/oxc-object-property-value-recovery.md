# Oxc missing object-property value recovery

This is the third bounded editor-recovery slice after missing variable initializers and local
assignment right-hand sides. It extends the current JSON-shaped checker milestone without changing
normal parser behavior or the public `tsrs` boundary.

## Recovery contract

In `ParseMode::Editor`, after a parsed object property name and colon, a comma or the object
expression's closing brace is owned by the active object-property list. The parser inserts a
zero-width `MissingExpression` at that token, reports `TS1109`, and leaves the boundary unconsumed
so the delimited-list parser can preserve later properties and enclosing statements. Nested object
contexts follow the same rule.

Assignment expressions inside a property value use the same active object context, so
`{ value: target = , intact: 1 }` recovers on the assignment right-hand side without treating the
comma as part of the expression.

Normal mode retains its fatal behavior. Valid object literals produce the same AST in Normal and
Editor modes. Missing closing object delimiters and malformed property names are outside this slice
and remain part of the broader object-delimiter/list recovery work.

## Downstream policy

`MissingExpression` remains semantically inert. `tsrs` suppresses a type diagnostic for the
recovered property itself but continues structural checking for trustworthy sibling properties and
later declarations. The owned diagnostic boundary therefore returns the parse error together with
independent type errors, without inventing a type for the missing value.

Focused parser tests cover comma and closing-brace boundaries, nesting, assignment RHS reuse,
determinism/progress, normal-mode isolation, structural placement, and valid-input identity.
Semantic, NAPI inspection, conformance, LSP edit-sequence, benchmark, and named playground evidence
cover the integration path.

Against pinned TypeScript-Go `89d5d5b2849a0db0957065889ca58536fa6d2e4a`, the parse diagnostic
matches exactly at `TS1109@4:12`. The whole-compiler differential still reports the intentional
editor-policy difference: TypeScript-Go suppresses checker diagnostics after the syntax error,
whereas `tsrs` keeps the independent sibling and later `TS2322` diagnostics. This is not claimed as
whole-diagnostic parity; the recovery manifest will compare the parser recovery dimensions.
