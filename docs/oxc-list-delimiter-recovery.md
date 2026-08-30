# Oxc list-delimiter recovery

This Stage 2 increment recovers punctuation that is absent from object-expression property lists,
array-expression element lists, and ordinary or `new` call argument lists. It builds on the
explicit list contexts introduced by the object-value, array-operand, and call-argument increments.

## Supported edits

In opt-in `ParseMode::Editor`, the parser now:

- inserts a missing comma before an unambiguous next property, element, or argument;
- inserts a missing `}`, `]`, or `)` at EOF; and
- inserts a missing inner closer when the current `}`, `]`, or `)` belongs to an enclosing list.

The current token is never consumed for a synthetic separator or closer. A missing comma must be
followed by a parser-recognized element start, and parsing that element must advance the cursor. A
missing closer is accepted only at EOF or at a closing token owned by an enclosing list. Other
malformed input retains Oxc's normal fatal behavior.

Normal mode is unchanged. Sparse array holes remain ordinary elisions, and valid programs produce
neither diagnostics nor recovery metadata.

## Representation and diagnostics

Punctuation has no expression-shaped AST slot, so the recovered object, array, or call remains its
ordinary AST node. `ParserReturn::recoveries` contains an owned, zero-width `RecoveryEvent` for each
synthetic token:

- `MissingComma`;
- `MissingClosingBrace`;
- `MissingClosingBracket`; or
- `MissingClosingParenthesis`.

Missing commas and EOF closers use TypeScript-compatible `TS1005` at the insertion point. At an
enclosing closer, TypeScript-Go first diagnoses the absent list separator, so a non-empty inner
list reports `TS1005 ',' expected` while its recovery event still classifies the synthesized closer.
An empty inner object, array, or call uses `TS1136`, `TS1137`, or `TS1135`, respectively. Parser
checkpoints also checkpoint the recovery-event length, preventing speculative parses from leaking metadata.
The parse-only playground inspection exposes these events as flat recovery sites rooted at the
program while its structural tree remains the real typed AST.

## Checker policy

`check_source` treats parser recovery metadata as proof that a non-fatal parser diagnostic belongs
to an explicitly supported editor edit. It therefore builds Oxc semantics and checks the recovered
tree. No checker suppression is needed for the punctuation itself: every existing child expression
is source-backed and is checked normally. This allows wrong property values, array elements, call
arguments, and later declarations to remain visible alongside `TS1005`.

## Evidence

The focused Oxc parser tests cover normal-mode aborts, all three missing commas, statement/EOF
closers, nested mismatched closers, deterministic recovery, cursor progress, unchanged valid ASTs,
and unchanged array holes. Semantic tests prove later bindings and references survive without
inventing `MissingExpression` nodes. The playground NAPI test asserts the exact recovery kind,
diagnostic association, and semantic summary.

The `recovered_missing_list_commas` and `recovered_missing_list_closers` conformance fixtures assert
exact `TS1005` locations together with independent `TS2322` and `TS2345` diagnostics. The LSP edit
sequence removes only the three parse diagnostics after completing the punctuation. The
`editor_recovery_missing_list_delimiters` benchmark and `missing-list-delimiters` playground example
exercise the combined path.

The pinned TypeScript-Go differential matches the six conformance-fixture parser diagnostics
exactly in code, message, and insertion location. Its checker stops after those parse diagnostics;
the additional `tsrs` type diagnostics are the documented editor-mode checker-policy divergence.
Affected Oxc formatting, strict clippy, tests, AST generation, and allocation snapshots pass, as do
the full `tsrs` gates and WASM/frontend/browser checks. A benchmark run initially exposed lost
inlining in the refactored list loop; restoring its prior always-inline contract returned
`small_file` to its prior range and removed the immediate object/array regression.

This increment does not recover malformed property names, incomplete type annotations, parameter
lists, or arbitrary skipped tokens. Those remain separate grammar and cascade-policy work.
