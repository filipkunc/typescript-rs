# Oxc editor recovery fork plan

Status: proposed architecture work, not yet started.

## Decision

`tsrs` should develop an Oxc fork with editor-grade parser recovery modeled on
TypeScript-Go. The fork is intended to remove the current parser boundary that prevents the
language server from checking intact parts of a document while another part is temporarily
incomplete.

This is upstream parser and AST work, not a diagnostic translation feature in `tsrs`. Recovery
that changes the structure available to the binder or checker belongs in Oxc. `tsrs` should keep
owning types and type diagnostics and should consume the recovered Oxc AST and semantic graph
through the existing single-check lifetime boundary.

The work should begin in a separate Oxc fork so it can be tested coherently across the parser,
AST, visitors, and semantic builder. Changes should remain reviewable and suitable for proposing
upstream as small independent improvements where possible.

## Motivation

Oxc already distinguishes recoverable and fatal parser errors. A recoverable error returns a full
AST, but a fatal error advances the lexer to EOF and ultimately replaces the partially constructed
program with an empty dummy program. That is a sensible batch-tool optimization, but it loses the
surrounding structure needed by an editor.

TypeScript-Go follows a different recovery contract:

- parsing contexts describe the list or construct currently being parsed;
- missing expected tokens and identifiers are represented without aborting the parse;
- recovery searches for a token that can resume the current or an enclosing context;
- recovery must always consume input or leave through a known boundary;
- the parser marks erroneous nodes and continues constructing the source file;
- the binder and checker can still operate on unaffected declarations.

The relevant reference implementation is
[`internal/parser/parser.go`](https://github.com/microsoft/typescript-go/blob/main/internal/parser/parser.go).
Oxc's current fatal-error design was introduced deliberately by
[#10579](https://github.com/oxc-project/oxc/pull/10579) and
[#10588](https://github.com/oxc-project/oxc/pull/10588). The fork therefore needs an explicit
editor mode rather than silently changing the optimized behavior expected by Oxc's existing batch
consumers.

## Goal

Given an incomplete TypeScript document, the editor parser should return the maximum structurally
trustworthy AST and syntax diagnostics while preserving later and enclosing valid constructs. Oxc
semantic construction and `tsrs` checking should be able to process the trustworthy portions
without panics, invalid spans, invented symbols, or diagnostic cascades.

The first target is the syntax used by current and near-term `tsrs` milestones:

- variable declarations, initializers, and assignment expressions;
- primitive, literal, union, object, and array types and expressions;
- function declarations, calls, parameters, and return statements;
- interfaces and member access;
- classes when the checker milestone reaches them.

Recovery should expand by explicit grammar area and fixtures. Full TypeScript recovery parity is a
long-term direction, not a prerequisite for resuming checker work.

## Non-goals

- Replacing Oxc with a separate parser inside `tsrs`.
- Copying the TypeScript-Go parser or AST mechanically.
- Requiring Oxc's normal transform, minifier, formatter, or linter paths to accept recovered ASTs.
- Implementing fine-grained incremental parsing in the first recovery milestone.
- Producing type diagnostics from structurally untrustworthy recovered nodes.
- Hiding missing parser structure through message-only rewrites in `tsrs`.

## Meaning of parity

Oxc and TypeScript-Go have different AST representations. TypeScript-Go can represent a missing
construct as a zero-width node of the expected kind in its unified node model. Oxc uses strongly
typed AST structs and enums, so the safest representation may require explicit missing/error
variants or recovery metadata.

For this work, **recovery AST parity** means observable behavioral parity:

1. the same valid outer and later declarations survive;
2. the parser identifies the same kind of missing or malformed construct;
3. recovery resumes at an equivalent grammar boundary;
4. missing constructs have stable zero-width or source-backed ranges;
5. parser diagnostics point at equivalent locations and avoid obvious cascades;
6. unaffected scopes, symbols, and references are still constructed;
7. downstream checking of unaffected code produces the same results.

It does not require identical node names, child layouts, flags, or diagnostic wording inside the
Oxc fork. TypeScript-compatible codes and final messages remain a separate `tsrs` compatibility
concern.

## Required parser model

### Opt-in editor mode

Add a parser option such as `ParseMode::Editor` or `enable_editor_recovery`. The exact API should
be decided in the fork after inspecting the current Oxc configuration conventions.

- Normal mode must preserve Oxc's current success path and fatal-error behavior.
- Editor mode may perform additional recovery work and return a partial/recovered program.
- Resource limits, excessive nesting, and internal invariant failures may remain fatal.
- `ParserReturn` must make it possible to distinguish clean, recovered, and fatally aborted parses.

### Recovery contexts

Model the TypeScript-Go parsing-context idea rather than using one global skip-to-semicolon rule.
Initial contexts should include:

- source elements and block statements;
- variable declarations;
- parameters and arguments;
- array and object elements;
- type members and class members;
- type arguments, type parameters, and union constituents.

Each context needs:

- valid element starters;
- valid terminators;
- tokens that belong to an enclosing context;
- a context-specific diagnostic when no element can be parsed.

Recovery should stop when the current token can begin an element or terminate either the current
context or an enclosing one. If none applies, it must consume a token before retrying.

### Missing and malformed syntax

Before selecting an AST representation, prototype the following alternatives against Oxc's
generated visitors and semantic builder:

1. expected-kind nodes with zero-width spans plus explicit recovery metadata;
2. dedicated missing/error variants in the relevant typed enums;
3. a side table in `ParserReturn` describing synthesized or skipped syntax.

The representation must satisfy these invariants:

- source-backed spans are ordered and within the source;
- missing spans are zero-width at the insertion point;
- skipped tokens retain a diagnostic range;
- visitors cannot mistake a synthesized identifier for a real declaration;
- semantic construction either ignores a recovery node or gives it an explicit safe meaning;
- valid-source AST layout and allocation behavior remain unchanged where practical.

The algorithms should be learned from TypeScript-Go, but the representation should fit Oxc rather
than forcing TypeScript-Go's unified-node design onto typed Rust enums.

### Error propagation

Recovered nodes need an observable error marker, either on nodes or through a range/node side
table. Parser and semantic consumers need helpers equivalent to:

- this node was synthesized or directly contains a parse error;
- this subtree contains recovered syntax;
- diagnostics have already been reported for this recovery site.

`tsrs` should map expressions that cannot have a trustworthy type to its error/unknown path and
suppress dependent diagnostics. It should continue checking independent declarations and valid
subexpressions.

## Fork and dependency workflow

1. Record the exact Oxc revision currently used by `tsrs` and create the fork from that compatible
   revision or from a deliberately reviewed upgrade.
2. Keep a dedicated recovery branch in the fork. Do not build unrelated checker behavior into it.
3. During local development, use Cargo's path patching against a sibling checkout.
4. Once a tested fork revision exists, pin every Oxc dependency to an exact Git revision; do not
   depend on a moving branch.
5. Keep a small compatibility note containing the Oxc base revision, fork revision, and required
   `tsrs` changes.
6. Regularly rebase or merge upstream in small intervals and rerun both Oxc and `tsrs` validation.
7. Submit generally useful recovery primitives and narrow grammar fixes upstream when they can be
   separated cleanly. The fork remains the integration vehicle until upstream support is complete.

The fork repository name and hosting location are intentionally undecided until implementation
starts. No `Cargo.toml` dependency should change before an actual tested revision is available.

## Test strategy

### Reference corpus

Create a parser-recovery corpus derived from pinned TypeScript-Go behavior. Start with complete,
small programs and generate meaningful edit states by truncating or deleting tokens. Include
handwritten cases where a malformed construct precedes valid code.

Initial examples should cover:

```ts
let
let x =
let x = "123"; x =
const value = {
const values = [1,
foo(
function f(value:
interface Box { value:
class Box { method(
const broken = ; const intact: number = "wrong";
```

Expand this into narrow fixtures rather than one large editing transcript.

### Recovery manifest

Exact serialized AST comparison would mostly measure representation differences. Instead, produce
a compact recovery manifest for both parsers containing:

- surviving declaration and statement kinds;
- parent/child structure relevant to the malformed region;
- source ranges;
- missing or error construct classifications;
- syntax diagnostic locations;
- surviving declaration names and semantic bindings where available.

A small reference probe inside a pinned TypeScript-Go checkout may use its internal parser to
generate these manifests. Generated reference data must record the exact TypeScript-Go revision.
The probe design must not turn normal tests into a network dependency.

### Oxc fork tests

Every recovery fixture must assert:

- termination and lexer progress;
- no panic;
- no reversed or out-of-bounds spans;
- deterministic diagnostics;
- preservation of expected surrounding AST nodes;
- successful traversal by generated visitors;
- safe semantic construction for supported recovery nodes;
- unchanged output for the corresponding complete valid program.

Fuzz and mutation tests should delete, duplicate, or replace tokens around delimiters and list
boundaries. Failures should be reduced into permanent focused fixtures.

### `tsrs` integration tests

Mirror the editor-relevant cases in `tsrs` only after the fork exposes the required structure.
Integration tests should prove that:

- incomplete syntax receives the intended parser diagnostic;
- a valid declaration after malformed syntax is still type-checked;
- existing symbols before the edit remain usable where TypeScript-Go keeps them;
- recovered expressions do not cause secondary type errors;
- LSP diagnostics clear or change correctly as each edit completes the construct.

Message normalization must not be used to make a structurally failed parse appear recovered.

## Implementation stages

### Stage 0: Baseline and research

- Pin the Oxc and TypeScript-Go reference revisions.
- Document Oxc's current recoverable and fatal paths.
- Extract the TypeScript-Go parsing contexts, list terminators, missing-node creation, and
  parse-error propagation relevant to the first grammar slice.
- Establish the recovery manifest format and baseline the initial fixture corpus.

### Stage 1: Recovery infrastructure

- Add the opt-in editor mode.
- Add recovery-context tracking and the progress invariant.
- Select and implement the minimum missing/error representation.
- Preserve a recovered `Program` instead of replacing it with `Program::dummy` in editor mode.
- Add span, visitor, and semantic-safety invariant tests.

No `tsrs` dependency switch should happen at this stage unless the infrastructure already improves
an end-to-end fixture.

### Stage 2: Current checker grammar

Implement context-aware recovery for variable declarations, assignment expressions, object and
array literals, type annotations, and delimiters. This stage should make the existing basic
expression and JSON-shaped milestones usable through common incomplete edits.

Switch `tsrs` to an exact fork revision only when the end-to-end corpus demonstrates a clear gain
and all existing conformance tests remain stable.

### Stage 3: Functions and interfaces

Add parameters, arguments, function bodies, return types, interface members, and member-access
recovery. Harden Oxc semantic construction and `tsrs` cascade suppression for these nodes.

### Stage 4: Classes

Add class-member recovery alongside the planned class type-checker milestone. Keep class instance
and constructor/static-side semantics in `tsrs`; the fork should provide only trustworthy syntax
and semantic identities.

### Stage 5: Broader parity and upstreaming

- Grow the edit-state corpus from TypeScript-Go conformance cases and real editor sessions.
- Upstream isolated recovery primitives and grammar fixes.
- Measure whether fine-grained incremental parsing is necessary after full-file recovery is stable.
- Reevaluate whether the fork remains necessary after upstream adoption.

## Completion criteria for the first usable milestone

The first Oxc recovery milestone is complete when:

1. supported incomplete-input fixtures no longer return an empty program in editor mode;
2. valid declarations following the malformed construct survive and are semantically bound;
3. the recovery manifest agrees with TypeScript-Go on the defined parity dimensions;
4. Oxc visitors and semantic analysis do not panic on the recovered corpus;
5. `tsrs` checks unaffected code and suppresses diagnostics derived only from missing syntax;
6. the LSP edit sequence is covered by automated tests;
7. Oxc's normal parse mode and valid-source AST behavior remain unchanged;
8. Oxc and `tsrs` formatting, linting, tests, documentation, and relevant benchmarks pass.

## Performance checks

Recovery is primarily an erroneous-input path, but its plumbing must not impose an unmeasured cost
on valid files.

- Benchmark normal mode before and after each infrastructure change.
- Benchmark editor mode on valid source and representative incomplete source.
- Track system and arena allocations separately where Oxc already provides that tooling.
- Keep recovery-context storage lazy if possible.
- Prefer explicit cold paths, but not at the cost of losing parser progress or valid structure.

`tsrs` should rerun `cargo bench --bench check_file` when adopting a new fork revision, with a
focused incomplete-document case added once recovered checking is enabled.

## Risks and decision points

- **AST breadth:** adding error variants can affect generated visitors and every exhaustive match.
  Resolve this with a small prototype before committing to a representation.
- **Semantic assumptions:** Oxc semantics currently expects structurally valid nodes. Each recovered
  node needs an explicit bind, ignore, or propagate policy.
- **Upstream drift:** a long-lived fork can become expensive. Keep commits narrow and the dependency
  pinned, and upstream reusable pieces early.
- **False parity:** matching diagnostic text while discarding structure is not recovery parity. The
  manifest and downstream-binding assertions guard against this.
- **Cascade noise:** continuing after syntax errors can produce misleading type diagnostics. Error
  marking and an error/unknown type path are part of the milestone, not cleanup work.
- **Scope growth:** TypeScript's grammar is large. Recovery follows checker milestones and focused
  fixtures rather than attempting full coverage immediately.

## Rule for continuing checker work

Checker features that can be implemented and tested on complete ASTs may continue independently.
When an editor scenario is blocked because Oxc discards or corrupts the required structure, add a
recovery fixture and fix the parser fork rather than adding another syntax-shape diagnostic rewrite
to `tsrs`.

This keeps the ownership boundary clear: Oxc supplies a useful, safe recovered syntax and semantic
model; `tsrs` supplies TypeScript types and type-system behavior.
