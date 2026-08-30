# Oxc function and interface recovery

This Stage 3 increment extends opt-in editor recovery across the function and interface syntax
that `tsrs` currently consumes. It preserves trustworthy parameters, bodies, type members, member
objects, and independent declarations while keeping normal Oxc parsing unchanged.

## Supported edits

In `ParseMode::Editor`, the parser now recovers:

- a missing parameter comma, a safe missing closing `)`, and an empty comma-delimited parameter
  slot;
- a missing parameter or ambient return type through the existing zero-width `MissingType`;
- a missing function-body `}` at EOF;
- a missing binary operand in a return expression at an owned block boundary;
- a missing interface-member type, same-line member separator, or interface closing `}` at EOF;
  and
- a missing static or optional member name, such as `box.;` or `box?.;`, at an owned expression
  boundary.

Missing parameters use `MissingParameter` metadata instead of a dummy binding. Missing commas,
semicolons, and closers likewise remain zero-width metadata. A missing member name uses the
internal `MissingMemberExpression`: it retains the real object expression and full expression span
for semantic traversal, plus a separate zero-width `missing_property_span` for diagnostics and
navigation. No empty identifier or property symbol is invented.

The supported diagnostics match the pinned TypeScript-Go parser: `TS1003` for a missing member
name, `TS1005` for separators and closers, `TS1109` for a missing return-expression operand,
`TS1110` for a missing type, and `TS1138` for a missing parameter declaration.

## Downstream policy

Oxc semantic construction visits the object of `MissingMemberExpression`, so real identifier
references survive, while no property identifier is bound. Empty parameter slots add no formal
parameter or semantic binding. `tsrs` receives parser recovery metadata for the duration of the
single `check_source` run and declines to register a callable signature containing a
`MissingParameter`; this prevents arity or argument diagnostics derived from an incomplete
signature. Missing member and expression nodes resolve to no checker type, while independent
declarations continue to check.

## Boundaries and exclusions

Recovery is boundary-driven, not token skipping. Parameter closers recover only at EOF or a token
owned by an enclosing construct; an opening function body is not guessed to terminate parameters.
Function and interface closing braces recover at EOF in this increment. Computed-member operands,
missing declaration names, arbitrary skipped tokens, generic type-parameter lists, and malformed
interface member names remain separate future work. Complete valid input has the same AST in
normal and editor modes.

## Evidence

Focused Oxc parser tests cover normal-mode behavior, deterministic zero-width sites, valid-AST
identity, parameter/member counts, and source-backed object preservation. Semantic and playground
inspection tests prove that surviving bindings and member-object references remain available and
that no missing parameter binding is invented.

Named `tsrs` conformance fixtures cover parameter slots and delimiters, function-body and return
expression recovery, return types, interface member types/separators/closers, and missing member
names. LSP edit sequences exercise recovered-to-complete transitions and retain independent
`TS2322` diagnostics. The offline recovery manifest compares all supported Stage 3 cases with the
pinned TypeScript-Go diagnostics, statement kinds, declarations, bindings, and recovery-site
counts. The `editor_recovery_function_interface_edits` benchmark and `function-interface-edits`
playground example exercise the combined path.
