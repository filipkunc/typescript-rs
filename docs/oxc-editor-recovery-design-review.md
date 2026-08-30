# Oxc editor-recovery design review

## Purpose

This note is the discussion checkpoint before proposing changes to the original Oxc repository. It
summarizes the fork's contract, the evidence collected in `typescript-rs`, the known limitation
found by a larger editor trace, and the questions that need maintainer input. It is not a proposal
to upstream the complete fork as one pull request.

## Proposed contract

The fork adds an opt-in `ParseMode::Editor`; normal mode remains the default. Editor mode may retain
locally incomplete syntax only when the active grammar context owns a safe recovery boundary.

The contract has five invariants:

1. Valid input produces the same AST in normal and editor modes.
2. Normal-mode behavior does not change.
3. Missing syntax is explicit: expression/type/member child slots use dedicated recovery AST nodes,
   while missing punctuation and empty declaration/parameter slots use owned parser metadata.
4. Recovery nodes and events are semantically inert. They do not invent symbols, references, or
   trustworthy types.
5. Every recovery consumes input or exits at a context-owned boundary; tests require deterministic,
   bounded spans and parser progress.

`tsrs` keeps the arena-backed AST and recovery metadata inside one `check_source` invocation. Only
owned diagnostics with UTF-8 byte ranges cross the public boundary.

## Evidence collected

| Area | Evidence | Result |
| --- | --- | --- |
| Cross-parser behavior | 25 checked-in TypeScript-Go edit-state cases | Exact parser/binder manifest parity |
| Recovery safety | 12 single-token deletion mutations | Deterministic AST, diagnostics, recovery sites, bounded spans, and parser progress |
| Valid/normal behavior | Focused normal-mode and valid-input equivalence tests for each recovery family | No intended behavior change outside editor mode |
| End-to-end editor behavior | Seven full-document LSP snapshots over one deterministic application-shaped TypeScript file | Both distant `TS2322` sentinels survive every incomplete and repaired snapshot |
| Workload size | Generated service-configuration source | More than 30 KiB and 1,300 lines |
| Complete snapshot | Criterion `parse_bind_check/editor_trace_large_complete` | 692–707 µs (95% estimate interval) |
| Recovered property snapshot | Criterion `parse_bind_check/editor_trace_large_recovered_property` | 740–759 µs (95% estimate interval) |
| Seven-snapshot sequence | Criterion `parse_bind_check/editor_trace_large_sequence` | 5.43–5.49 ms (95% estimate interval) |
| Fork integration | Oxc PR 2, playground PR 2, and `typescript-rs` PR 5 | Fork CI and integration CI passed; all three PRs merged |

The Criterion measurements were taken on the Fedora development machine on 2026-08-30 using the
release benchmark profile. They are local decision evidence, not portable performance promises.
The workload is deterministic and application-shaped, not a captured trace from an external
codebase. A genuinely recorded trace remains useful future evidence before making incremental
parsing decisions.

The seven snapshots are:

1. complete source;
2. object property value deleted;
3. property repaired;
4. call closer deleted before the next declaration;
5. call repaired;
6. declaration name deleted;
7. declaration repaired.

Parser diagnostics precede checker diagnostics in the published list. The missing call also yields
the expected dependent arity diagnostic, while independent diagnostics before and after the edit
remain available.

## Limitation found by the larger trace

The supported missing-call-closer rule recovers when the following declaration is the safe outer
boundary. An exploratory variant deleted `)` directly around a nested object argument, leaving a
shape like this:

```ts
selectService({
    endpoint: { host: "editor.internal", port: 9000 },
};
const later: number = "wrong";
```

That snapshot aborted the current full pipeline and produced `TSRS1000`; it did not preserve the
distant checker diagnostics. The passing trace therefore uses the narrower boundary the fork
currently claims to support. Before broadening this grammar area, we should agree with Oxc
maintainers which context owns the mismatched `}`/missing `)` pair and what AST should survive.
This should not be hidden by adding a broad token-skipping fallback.

## Suggested upstream sequence

Do not open the complete integration checkpoint as a large upstream pull request. After design
feedback, derive the smallest accepted slice from current Oxc `main`:

1. Add the opt-in mode, recovery-context/metadata contract, invariants, and tests without enabling
   broad new grammar recovery.
2. Add one context-owned recovery family with its generated AST/visitor changes (if any), semantic
   inertness, normal-mode tests, and allocation snapshot in the same reviewable change.
3. Re-run the TypeScript-Go manifest externally and attach the comparison as evidence; keep the
   upstream test suite network-independent.
4. Continue one grammar family at a time only after the ownership model is accepted.
5. Treat NAPI inspection and playground presentation as later integration layers, not parser API
   prerequisites.

Candidate first grammar slices are the missing initializer/assignment RHS because their owner and
boundary are narrow, or list punctuation because it validates the shared ownership primitive. The
maintainers' preference should decide between them.

## Questions for Oxc maintainers

1. Is an opt-in parser mode the right compatibility boundary, or would Oxc prefer recovery policy
   to be expressed through another parser option/API?
2. For child-shaped holes, are explicit missing AST variants acceptable, or should all recovery be
   represented outside the AST? What downstream visitor guarantees are required?
3. Is owned `ParserReturn` metadata appropriate for missing punctuation and empty slots that have
   no honest AST child, or is there an existing diagnostics/event model we should extend?
4. Which invariants should be required for editor recovery beyond unchanged normal mode, unchanged
   valid ASTs, semantic inertness, deterministic spans, and guaranteed progress?
5. Which first grammar slice would be easiest to review and most useful to Oxc consumers?
6. Is there active or planned parser-recovery/incremental-parser work whose ownership model this
   prototype should align with before code is proposed?
7. For the nested object-argument limitation above, should the call context own recovery at `}` or
   should the parser intentionally stop because ownership is ambiguous?

## Discord discussion draft

> Hi! I have been prototyping opt-in TypeScript editor recovery in an Oxc fork while building a
> small checker on top of Oxc. Before opening any upstream PR, I would like feedback on the design
> boundary and on how to split it into a genuinely small first change.
>
> The prototype keeps normal parsing as the default and adds an editor mode. It uses explicit
> missing AST nodes only where a real child slot exists; missing punctuation/empty slots are owned
> recovery metadata. Recovery nodes are semantically inert, valid input has normal/editor AST
> equivalence tests, and every recovery boundary has deterministic-progress/span tests.
>
> Current evidence: exact parity on a checked 27-case TypeScript-Go recovery manifest, a 12-case
> deletion matrix, and a seven-version 30+ KiB LSP trace that preserves type diagnostics before and
> after each incomplete edit. Full parse-bind-check of that source is about 0.70 ms locally; the
> recovered-property version is about 0.75 ms. I also found a limitation: deleting `)` around a
> nested object call argument is still ambiguous and aborts the current pipeline, so I do not want
> to broaden recovery there without agreeing on context ownership.
>
> Would `ParseMode::Editor` plus owned recovery metadata be a reasonable Oxc API direction? For
> syntax holes, would you prefer explicit AST variants or a side table? And for a first PR, would a
> contract/primitives-only slice, a narrow initializer RHS slice, or a list-ownership slice be most
> reviewable? I can share the design/evidence note and fork commits, but I do not intend to submit
> the full prototype as one PR.
