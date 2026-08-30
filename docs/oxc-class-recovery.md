# Oxc class recovery

This Stage 4 increment adds a class-body recovery context after the first supported class checker
milestone. Oxc continues to own only syntax, scopes, symbols, references, and recovery metadata;
the distinct instance and constructor/static sides remain `tsrs` types and signatures.

## Supported edits

In `ParseMode::Editor`, a same-line missing separator between class properties records a zero-width
`MissingSemicolon`, emits TypeScript-compatible `TS1005`, and retains both real property nodes. A
class body missing its closing `}` at EOF records `MissingClosingBrace` and emits `TS1005` without
discarding the class declaration.

`RecoveryContext::ClassMembers` scopes those decisions to the owning class list. The class parser
uses the shared recoverable list/closer operation, while its existing property terminator path
records only the missing punctuation. Normal mode remains fatal for the missing same-line
separator, and valid class input has identical normal/editor ASTs.

## Downstream policy

Both recoveries are metadata-only: no member name, value, symbol, or type is invented. Oxc semantic
construction binds the real class and members. The checker can therefore build its supported
instance/static shapes, resolve `new` and member access, and keep independent type diagnostics.
Completing the separator or closer removes only its parse diagnostic on the next full-document
check.

Arbitrary malformed member names, modifier sequences, computed/private members, skipped tokens,
and a method whose unfinished parameter list consumes later source remain outside this bounded
slice.

## Evidence

Focused parser tests cover normal-mode failure, separator and EOF-closer recovery, exact zero-width
events, member counts, and valid-AST identity. Semantic and inspection tests prove class and
neighboring bindings survive. Named conformance and LSP cases preserve class checking and an
independent `TS2322`; the pinned recovery manifest compares both class edit states exactly with
TypeScript-Go. `editor_recovery_class_member` and the `missing-class-member-separator` playground
example exercise the combined separator/closer path.
