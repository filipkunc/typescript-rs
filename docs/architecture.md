# Bootstrap architecture

## Boundaries

The initial pipeline is deliberately a single crate. Crate boundaries are expensive to
change and the type representation, project model, and incremental strategy are not yet
stable enough to justify them.

- Oxc owns scanning, parsing, its arena-backed AST, scopes, symbols, references, and
  syntax diagnostics.
- `tsrs` owns TypeScript types, assignability, inference, type diagnostics, and eventually
  program/project state.
- The checker borrows Oxc's `Scoping` for one `check_source` run. Declaration `SymbolId`s and
  expression `ReferenceId`s connect syntax to tsrs-owned type and signature tables without
  copying or persisting Oxc semantic data.
- The public boundary returns owned diagnostics. Neither Oxc AST references nor allocator
  lifetimes escape a single-file check.
- The CLI is only a frontend. Checker behavior is tested through the library.

The Oxc source baseline is pinned as the `vendor/oxc` Git submodule so editor-recovery changes to
the AST, parser, generated visitors, and semantic builder can be developed and tested together.
Cargo resolves every Oxc crate from that checkout; normal parser behavior remains the baseline
and the first opt-in recovery slice described in the [recovery plan](oxc-editor-recovery-plan.md)
preserves a missing variable initializer as `MissingExpression`. The next local slice adds explicit
source/block recovery contexts and preserves a missing assignment right-hand side at boundaries
owned by those contexts. The object-value slice adds an object-property list context, leaves commas
and closing braces to that owner, and skips only the recovered property's type relation so sibling
properties remain checkable. The array-element context applies the same rule to assignment and
spread operands at commas/closing brackets while retaining ordinary elisions for array holes. The
argument context also represents an empty comma-delimited argument as `MissingExpression` with
`TS1135`, preserving later arguments and their parameter checks. The three list contexts share
missing-comma and safe closer recovery. Because punctuation has no AST child slot, Oxc returns an
owned zero-width recovery event alongside the ordinary object, array, or call node; `check_source`
uses that metadata to distinguish supported non-fatal syntax from unrelated parser errors.
An absent annotation type instead occupies a typed child slot and is represented by the explicit
zero-width `MissingType` variant. Missing array, parenthesized, and type-argument closers extend the
same recovery metadata with `]`, `)`, and `>` insertion sites. `tsrs` leaves top-level missing
types unresolved and lowers a directly missing object-property type to its existing `any` identity
to suppress dependent noise without hiding source-backed sibling diagnostics.
The final Stage 2 representation, `MalformedExpression`, carries the non-empty span of an
unexpected `:` or `...` token in a source/block initializer or assignment slot. It is semantically
inert and resolves to no checker type, keeping it distinct from both a zero-width missing node and
trustworthy source-backed expressions.
Stage 3 adds parameter and type-member recovery contexts. Empty parameter slots are recorded as
owned `MissingParameter` events and create no binding; `check_source` keeps those events alive only
for its current parse/check run, and the checker skips a callable signature containing one.
Missing static or optional member names use `MissingMemberExpression`, which owns the real object
expression and a distinct zero-width property insertion span. Semantic traversal therefore keeps
the object reference without inventing a property identifier. Function-body, parameter,
interface-member, return-expression, and interface-closing recovery follow the bounded policy in
[`oxc-function-interface-recovery.md`](oxc-function-interface-recovery.md).
Stage 4 keeps annotated callable members as structural function identities backed by the existing
signature store. Supported classes use a structural object identity for their instance side and a
distinct named constructor/static identity; the class value symbol maps to the latter, while a type
reference and `new` resolve to the former. Oxc's `ClassMembers` recovery context retains real class
across missing separators and records an EOF closer as punctuation metadata, as specified in
[`class-checker-milestone.md`](class-checker-milestone.md) and
[`oxc-class-recovery.md`](oxc-class-recovery.md).
Stage 5 adds a declaration boundary for an unclosed call followed by `const` or `var`, and preserves
a statement-position variable declaration missing its name as an empty declarator list. Two owned
recovery events retain the exact parser diagnostics without manufacturing a binding. The complete
25-case manifest, deletion matrix, LSP edit sequence, upstream slicing, and decision to defer
fine-grained incremental parsing are recorded in
[`oxc-recovery-upstreaming.md`](oxc-recovery-upstreaming.md).
The larger seven-version editor trace, its performance evidence, known nested-call limitation, and
maintainer questions are consolidated in
[`oxc-editor-recovery-design-review.md`](oxc-editor-recovery-design-review.md).
`check_source` opts into editor mode, keeps the syntax diagnostic, and checks independent syntax in
the recovered program while
treating the missing expression as having no trustworthy type. Exact pinned fork and upstream
revisions are recorded in the
[compatibility note](oxc-fork.md).

Cross-parser recovery parity is stored as owned JSON described in the
[recovery-manifest workflow](oxc-recovery-manifest.md). A checked-in probe runs only during explicit
regeneration inside the pinned TypeScript-Go checkout. Normal Rust tests validate the revision and
case sources, then compare Oxc diagnostics, normalized surviving statement kinds, declaration
names, semantic bindings, and explicit recovery-site counts entirely offline.

`check_source` currently runs all stages serially for one file. A future `Program` should
own source snapshots and stable file identities, and should schedule independent files in
parallel only after module resolution and dependency tracking exist.

## Near-term type model

Grow the checker around explicit operations instead of mirroring the monolithic upstream
checker:

1. interned primitive and literal type identities with canonical unions (initial support is in place);
2. annotation resolution (initial top-level, non-generic aliases and property-only interfaces are
   in place);
3. assignability with recursion guards and relation caches (initial support is in place);
4. object types and arrays (initial named-property, `T[]`, and built-in `Array<T>` support is in
   place), followed by tuples and index-signature types;
5. expression inference and contextual typing (initial object, array, and union-target support is
   in place), followed by control-flow narrowing;
6. signatures (initial explicitly annotated function declarations and direct calls are in place),
   with explicitly annotated callable expressions inferred through simple variables now using the
   same function identities and signature store, followed by generics, intersections, and
   conditional types.

Each operation should be introduced by focused conformance cases and measured against the
existing benchmark before broader upstream suites are enabled.

## Completed checker milestone: property-only interfaces

This checker milestone adds named interfaces as an alternate declaration form for the existing
structural object model. It includes unique top-level interfaces, including named and default
exports, whose bodies contain required or optional statically named property signatures with
explicit annotations. Interface references work wherever the current checker resolves annotations,
including variable declarations and explicitly annotated function parameters and return types.

Interface bodies lower to canonical `TypeKind::Object` identities, so aliases, interfaces, and
object type literals with the same property shape share assignability and contextual typing. The
checker retains borrowed declaration metadata only for the current check so nested missing, excess,
and wrongly typed property diagnostics can preserve interface names and locate the relevant syntax.
This also lets the completed annotated-callable foundation pass and return whole interface values
without adding callable identity to `TypeKind`.

The milestone excludes interface inheritance and declaration merging; recursive and generic
interfaces; method, call, construct, and index signatures; property/member access expressions; and
classes. Those later callable-member and bounded class-side increments are now documented below.

## Completed checker slice: basic expressions

Simple identifier bindings now populate the existing symbol-to-type table from an explicit
annotation or a supported initializer expression. Plain `=` expressions resolve their left-hand
identifier through Oxc's reference graph and apply the existing assignability and structural
diagnostic operations to the right-hand side. This is declaration-type checking, not flow-sensitive
state: a successful assignment does not replace or narrow the variable's type.

This slice intentionally excludes compound and destructuring assignments, use-before-declaration,
definite-assignment analysis, reassignment of properties or indexed values, and control-flow
narrowing. Fatal parser diagnostics still originate in Oxc. The owned-diagnostic boundary only
normalizes two unambiguous, common incomplete-input shapes to TypeScript-compatible `TS1109` and
`TS1128` codes and messages for the editor loop.

## Current tooling milestone: live single-document diagnostics

The `--lsp` frontend provides the first editor-facing loop without introducing a premature
project model. A Tokio stdio server built on `tower-lsp-server` owns full text snapshots and version
numbers only for documents opened by the client. On open and change it invokes the existing
`check_source` boundary and publishes owned diagnostics; on close it removes the snapshot and
clears diagnostics. Requests are handled sequentially for now so an older check cannot publish
after a newer document version. For the supported recovery shapes, a missing variable initializer,
assignment right-hand side, or object/array/call operand no longer hides independent type
diagnostics. The same applies to supported object/array/call list delimiters, and completing the
expression, punctuation, annotation, type closer, or supported source-backed malformed token
removes its syntax diagnostic on the next full-document check.
Supported parameter, function-body, return-expression, interface, and member-access edits use the
same full-document recovery loop and retain independent diagnostics.
An application-shaped 30+ KiB trace exercises seven complete snapshots and asserts that persistent
diagnostics near the beginning and end of the file survive local deletion and repair edits.

The LSP adapter is binary-only frontend code. It translates UTF-8 byte ranges to UTF-16 line and
character positions and does not move Oxc arenas, AST nodes, scopes, or references across checks.
The development VS Code extension is a thin standard LSP client for exercising this same stdio
server. Other clients do not depend on it.

This tooling milestone does not yet define `Program`: it neither scans a workspace nor reads
`tsconfig.json`, watches unopened files, resolves modules, or caches semantic state. Project-wide
diagnostics and language features must wait for stable file identities, configuration ownership,
module resolution, and dependency tracking.

## Completed checker milestone: callable members and class sides

Statically named interface property reads and explicitly annotated method calls now provide the
semantic path shared by class instances. The bounded class slice models instance and
constructor/static sides separately and is specified in
[`class-checker-milestone.md`](class-checker-milestone.md). Broader class syntax and callable
structural relations remain later checker milestones rather than being accepted only for the editor
demo.

## Completed checker milestone: explicitly annotated callable expressions

Arrow and ordinary function expressions with required simply named annotated parameters and an
explicit return annotation now infer a canonical `TypeKind::Function` through a direct variable
initializer. Their diagnostic-rich signatures remain in `SignatureStore`, parameter symbols are
available while checking the body, and the variable symbol selects the same signature for direct
calls. Concise arrow bodies are checked as implicit returns; block bodies reuse return-statement
checking. Argument types and exact arity reuse the declaration-callable path.

This remains the bounded slice specified in
[`callable-expression-milestone.md`](callable-expression-milestone.md). It does not add contextual
typing, inferred return types, general function assignability, broader callable aliasing, or
control-flow return analysis.

## Diagnostics and editor use

Diagnostics store UTF-8 byte ranges because that is Oxc's native coordinate system. Structural
checks walk fresh object and array expressions contextually so missing or excess properties and
wrongly typed nested values can point at their relevant syntax. The LSP adapter translates these
ranges to UTF-16 positions at the boundary. Stable codes, owned messages, and
deterministic concise rendering support both editor clients and test baselines without coupling
checker internals to either frontend.

## Test synchronization

Each fixture has a small, explicitly named Rust test so rust-analyzer exposes individual
cases in VS Code's Test Explorer. A guard test detects source fixtures that have not been
registered. Source directives remain comments that can later be interpreted by a harness
options parser. Baselines are sidecars rather than inline assertions, making upstream
source updates easy to diff. Cargo's test-name filter keeps an individual case convenient
to run under a debugger; `BLESS=1` updates only selected baselines.

`cargo test-tsgo` provides a separate differential layer over those same `.ts` sources. It
requires the exact TypeScript-Go revision recorded in `tests/tsgo-reference.txt`, checks the
fixtures under one fixed compiler-options profile, and compares normalized diagnostic codes,
UTF-16 locations, and messages. The `.errors` files remain tsrs regression snapshots; the
differential runner neither consumes nor rewrites them. Keeping these two checks independent
makes a local baseline update unable to conceal drift from the reference implementation.
