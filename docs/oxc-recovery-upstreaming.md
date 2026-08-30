# Oxc recovery upstreaming and incremental-parsing decision

## Outcome

The local editor-recovery prototype now matches the pinned TypeScript-Go manifest on all 25 cases.
The final two grammar reductions preserve a call when a following declaration owns its missing
closer, and preserve `const = 1` as a nameless, empty declaration without inventing a semantic
binding. A deterministic deletion matrix covers twelve token removals, and a four-version LSP test
covers clean input, both new recovery shapes, and their repair.

The fork remains necessary today. The upstream Oxc baseline does not contain the opt-in editor
mode, recovery contexts, recovery AST variants, owned punctuation/declaration recovery metadata, or
their semantic-safety policy. Pinning cannot move to upstream until equivalent pieces are adopted.

## Upstream review slices

Submit the work as reviewable grammar-independent primitives followed by narrow grammar changes.
Do not mix `tsrs` type-checker behavior or playground UI into Oxc parser changes.

1. **Opt-in contract and owned metadata.** Introduce `ParseMode::Editor`, the recovery-context
   stack, `ParserReturn::recoveries`, span invariants, and unchanged-normal/valid-AST tests. This is
   the common boundary for later slices and does not itself broaden accepted grammar.
2. **Recoverable list and closer primitives.** Isolate delimiter ownership, missing separators,
   safe outer-boundary closers, and the progress assertion used by object, array, call, parameter,
   type-member, and class-member lists.
3. **Missing syntax representations.** Review `MissingExpression`, `MissingType`,
   `MalformedExpression`, and `MissingMemberExpression` separately where practical, with generated
   visitor/layout changes and semantic inertness in the same change as each representation.
4. **Grammar reductions.** Land initializer/assignment, object/array/call, type, function/interface,
   class, missing call-closer, and missing declaration-name behavior as focused changes. Each change
   carries its normal-mode, editor-mode, deterministic-progress, visitor, and semantic tests.
5. **Inspection boundary.** Propose the NAPI recovery inspection only after the parser-owned data
   model is accepted; keep the Vue presentation in the playground repository.

Every proposed slice must pass Oxc formatting, generation where applicable, strict affected-crate
Clippy, focused and full tests, parser conformance, allocation snapshots, and relevant benchmarks.
Unexpected normal-mode or valid-input changes block upstreaming. The checked TypeScript-Go
manifest remains the cross-parser review oracle; it is not copied into Oxc as a network-dependent
test.

The complete local fork passes the pinned Test262, Babel, and TypeScript parser and semantic
conformance runs. Parser snapshots intentionally gain `Opened here` secondary labels in eleven
Babel and five TypeScript unmatched-delimiter diagnostics; repeat runs are stable and no test
classification changes. Keep those reviewed snapshot updates with the delimiter-diagnostic slice.

The tested integration commits and pull requests are published only in the `filipkunc` Oxc and
playground forks, and `typescript-rs` records their exact revisions. No branch, pull request, or
write targets the original `oxc-project` repositories because that publication scope was not
authorized.

## Publication order

Publish the already-tested integration state in dependency order before constructing upstream
topic branches:

1. Review and commit the Oxc fork working tree on `feat/editor-missing-expression`, including the
   generated AST/visitor surfaces, focused tests, reviewed conformance snapshots, NAPI inspection,
   and the normal/editor parser benchmark. Push it and require fork CI to pass.
2. Commit the playground UI and browser smoke on `feat/editor-recovery-playground` against that
   exact Oxc revision, then push it and require playground CI to pass.
3. Advance both `tsrs` gitlinks, replace the pending-local compatibility text with the two exact
   commit IDs, rerun the root gates, and publish the integration branch.
4. Derive upstream topic branches from Oxc upstream `main` in the review-slice order above. Each
   branch must include its focused tests and generated output and must pass independently; do not
   present the large integration checkpoint as one upstream change.

The configured fork branches and their integration pull requests are published. The checkout has
no separate Oxc upstream remote. GitHub CLI authentication through the host keyring is available,
but original-project pull requests remain intentionally untouched.

## Incremental parsing decision

Fine-grained incremental parsing is deferred.

Repeated Criterion runs measure the complete parse-bind-check path for the new recovery shapes at
2.07–2.18 microseconds for the missing declaration name and 2.62–2.92 microseconds for the missing
call closer on this machine. Existing representative complete checks remain roughly 5–11
microseconds for the current small-file, JSON-shape, callable, and bounded-class fixtures. Recovery
therefore does not currently consume a meaningful fraction of an interactive diagnostic budget.
The fork-level parser benchmark independently measures about 1.55 microseconds for normal-valid
input, 1.62 microseconds for the identical editor-valid input, and 1.85 microseconds for a
representative incomplete editor document.

Incremental parsing would also require decisions the bootstrap architecture intentionally has not
made: stable file and node identities, arena lifetime ownership across snapshots, invalidation,
dependency tracking, and a `Program` API. Adding those only to avoid the measured full-file costs
would couple the checker to an unstable representation.

Revisit this decision after module resolution and the `Program` model exist, using recorded editor
traces over realistic files. Measure end-to-end p50/p95 parse-bind-check latency, allocation volume,
file-size scaling, and time discarded by superseded document versions. Consider fine-grained
incrementality only when those measurements identify parsing or semantic reconstruction—not LSP
transport, project discovery, or type relations—as a material bottleneck.

## Fork reevaluation

For now, keep the exact fork boundary and continue to return only owned diagnostics from
`check_source`. As upstream slices land, remove equivalent local changes one slice at a time, rerun
the 25-case manifest and all release gates, and compare the resulting normal/editor ASTs. The fork
can be retired only when upstream supplies every recovery representation and semantic guarantee
used by `tsrs`; diagnostic-text similarity alone is insufficient.
