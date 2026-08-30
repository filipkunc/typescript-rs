# Cross-parser recovery manifest

The recovery manifest records observable parser and binder behavior from the pinned
TypeScript-Go revision without making normal tests depend on Go, a TypeScript-Go checkout, or the
network. It is the reference boundary for comparing different typed AST representations.

## Files and schema

- `tests/recovery-manifest/cases/*.ts` contains one narrow edit state per file.
- `tools/tsgo-recovery-probe/main.go` is the checked-in probe source.
- `tests/recovery-manifest/typescript-go.json` is generated reference data with schema version,
  implementation, exact revision, source text, top-level statement kinds/ranges/names, declaration
  and binding names, parser diagnostics, and missing/direct-error node paths.
- `tests/recovery_manifest.rs` validates the snapshot offline and compares implemented Oxc editor
  recovery against the shared parity dimensions.

The raw TypeScript-Go recovery-node paths remain in the manifest for review, but they are not
required to match Oxc node-for-node. The automated comparison instead asserts exact diagnostic
code/message/start location, surviving normalized statement kinds, declaration names, semantic
bindings, and one explicit Oxc recovery node or punctuation site per diagnostic. This preserves the
plan's behavioral definition of parity while allowing TypeScript-Go's missing identifiers and
Oxc's `MissingExpression`, `MissingType`, `MalformedExpression`, and recovery metadata to differ.

## Current corpus

The 25-case corpus contains the eight completed Stage 2 areas: missing initializer, assignment RHS,
object value, array operand, call argument, list delimiters, type syntax, and source-backed malformed
expression. It also covers the implemented Stage 3 function and interface edits: parameter slots,
delimiters, and types; function-body closers; return operands and types; interface member types,
separators, and closers; and static or optional member names. Stage 4 adds class-member separators
and class-body closers.

All 25 cases participate in exact Oxc parity. The final Stage 5 additions cover a missing call
closer owned by a following declaration and a missing variable declaration name. The latter uses an
empty declarator list plus owned recovery metadata, so no placeholder identifier or binding is
invented.

## Regeneration

Use a clean checkout at the revision in `tests/tsgo-reference.txt` and Go 1.26 or newer:

```console
./scripts/generate-recovery-manifest /path/to/typescript-go
```

Set `GO=/path/to/go` when `go` is not on `PATH`. The script verifies the checkout revision, creates
a temporary package inside that checkout so Go's internal-package rule is satisfied, runs the
checked-in probe, removes the temporary package, and writes only the JSON reference file in this
repository. Dependency downloads may occur during explicit regeneration; normal `cargo test` does
not invoke the probe or access the network.

Treat a manifest diff like a reference-version or parser-contract change: review every diagnostic,
surviving node, recovery path, declaration, and binding. Updating the JSON cannot bless an Oxc
parity failure because the Rust test compares current Oxc behavior to the pinned reference.
