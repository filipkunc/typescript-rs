# Oxc type recovery

This Stage 2 increment preserves incomplete type syntax needed by the current `tsrs` annotation
model. It covers an absent type and safe missing closers for array, parenthesized, and built-in
generic type syntax.

## Supported edits

In opt-in `ParseMode::Editor`, the parser now:

- inserts a zero-width missing type at an annotation, alias, union constituent, or object-property
  type boundary;
- inserts a missing `]` after an array-type suffix at a safe enclosing boundary;
- inserts a missing `)` after a parenthesized type at a safe enclosing boundary; and
- inserts a missing `>` after type arguments at a safe enclosing boundary.

The missing-type boundaries are `=`, semicolon, comma, a closing delimiter, arrow, union or
intersection separator, colon, and EOF. The type-argument list deliberately does not invent a
comma: `Array<string number>` remains fatal because the pinned TypeScript-Go parser diagnoses a
missing `>` and then an expression error rather than continuing it as two type arguments. Tuple
semicolons and malformed type-literal separators likewise remain outside this slice.

Normal mode is unchanged, and complete valid type syntax produces neither a recovery node nor
recovery metadata.

## Representation and diagnostics

An absent type occupies a typed AST slot, so `TSType::MissingType` represents it with a zero-width
span and is excluded from ESTree serialization. Generated visitors traverse the variant safely;
the semantic builder treats it as inert. Oxc code generation and formatting emit no source for
the synthesized node, while declaration emit and decorator metadata use conservative unresolved
fallbacks.

Missing punctuation remains ordinary parser metadata. `ParserReturn::recoveries` records
`MissingClosingBracket`, `MissingClosingParenthesis`, or `MissingClosingAngleBracket` at the
insertion point. Missing types report TypeScript-compatible `TS1110 Type expected.` and missing
closers report `TS1005` with the expected token. Parser checkpoints truncate both speculative AST
state and recovery metadata.

## Checker policy

A missing top-level annotation or alias does not resolve to a `tsrs` type, preventing dependent
assignment diagnostics. A directly missing object-property annotation keeps the property in its
containing shape with the checker's existing `any` identity. This prevents a misleading excess-
property diagnostic while allowing source-backed sibling properties to be checked normally.
Independent declarations continue through the ordinary checker pipeline.

## Evidence

Focused Oxc parser tests cover normal-mode aborts, annotation/alias/union/property sites, multiple
missing union constituents, all three type closers, deterministic diagnostics, progress, unchanged
valid ASTs, and the compact `TSType` layout. Semantic and playground-inspection tests prove that
missing types are inert, later bindings survive, and both AST recovery nodes and punctuation sites
remain inspectable.

The `recovered_missing_type_annotation` and `recovered_missing_type_delimiters` conformance cases
assert exact parser and surviving checker diagnostics. The LSP edit sequence proves completing the
annotation and closer removes only `TS1110` and `TS1005`. The
`editor_recovery_missing_type` benchmark and `missing-type-annotation` playground example exercise
the combined path.

The pinned TypeScript-Go differential agrees on `TS1110` for all supported missing-type positions
and on the three `TS1005` closer diagnostics, including exact messages and locations. Its checker
stops after parser errors, while `tsrs` intentionally reports trustworthy sibling and later type
errors. Oxc formatting, generation, strict affected-package Clippy, parser/semantic/inspection
tests, allocation snapshots, the full `tsrs` gates, WASM/frontend checks, and browser smoke all
pass for this local increment. Source-backed malformed expressions are specified separately in
[`oxc-malformed-expression-recovery.md`](oxc-malformed-expression-recovery.md).
