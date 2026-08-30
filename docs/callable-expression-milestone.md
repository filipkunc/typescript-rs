# Checker milestone: explicitly annotated callable expressions

## Goal

Extend the existing annotated-callable foundation from named function declarations to callable
expressions whose complete type is written on the expression itself. The result is inferred through
a simply named variable and called directly through that variable. This is a checker increment, not
an expansion of parser ownership or a general function-type system.

## Supported slice

- synchronous, non-generator arrow function expressions;
- synchronous, non-generator ordinary function expressions, whether anonymous or locally named;
- required parameters that use a single binding identifier and have an explicit type annotation;
- an explicit return annotation;
- concise arrow expression bodies, arrow block bodies, and ordinary function block bodies;
- type inference for a simple variable initialized with one of these expressions;
- direct calls through that variable;
- parameter-symbol resolution in the body, return-value checking, argument-type checking, and exact
  arity checking.

The supported annotations are the same annotations already resolved by `tsrs`. Return checking is
limited to concise arrow bodies and source-backed `return` statements. Proving that every block path
returns remains a later control-flow milestone.

## Representation and checking flow

Callable expressions reuse `TypeKind::Function`, `TypeStore::function`, `Signature`, and
`SignatureStore`.

When a supported expression is used as a variable initializer, expression inference:

1. resolves every parameter and the return annotation;
2. interns the structural function identity in `TypeStore`;
3. stores the diagnostic-rich signature in `SignatureStore`;
4. records parameter symbol types for body checking; and
5. associates both the expression span and the variable symbol with that signature.

The normal visitor then installs the expression's signature while walking its body. Existing return
checking handles block statements; a concise arrow body is checked as its implicit returned value.
Existing identifier/reference resolution and call argument checking handle calls through the
variable. Signature registration is cached by expression span so repeated inference does not create
duplicate signatures.

No Oxc arena reference crosses `check_source`, and diagnostics remain owned UTF-8 byte ranges.

## Diagnostics

- `TS2322` for a parameter-derived or other returned value that is not assignable to the explicit
  return annotation;
- `TS2345` for an argument that is not assignable to its annotated parameter;
- `TS2554` when a direct call supplies anything other than the exact required argument count.

Diagnostics keep the same wording and anchoring rules as the declaration-callable baseline. A
concise arrow return mismatch points at its body expression.

## Explicit exclusions

- contextual typing, including an unannotated expression assigned to an annotated function type;
- inferred return types or unannotated parameters;
- generic functions, overloads, async functions, and generators;
- optional, default, rest, destructured, parameter-property, or `this` parameters;
- function-type annotation resolution and general function assignability;
- immediately invoked expressions, callable expressions nested in object or array initializers,
  arbitrary callee expressions, and aliasing beyond the directly initialized variable;
- closure analysis, captured-variable typing beyond existing identifier lookup, recursion through a
  function expression's local name, and broader control-flow or all-paths-return analysis.

Unsupported forms remain outside checker coverage rather than receiving speculative diagnostics.

## Conformance and recovery boundary

Focused fixtures cover the valid forms and each diagnostic family. They are compared with the pinned
TypeScript-Go revision when that checkout is available. Only after those semantic cases pass may
incomplete-arrow editor states be inspected. Recovery changes require their own bounded design and
must preserve the normal-parser, declaration-callable, class, interface, JSON-shape, and existing
editor-recovery baselines.

All four semantic fixtures match TypeScript-Go revision
`89d5d5b2849a0db0957065889ca58536fa6d2e4a` exactly through `cargo test-tsgo`: the valid forms,
parameter-derived return errors, argument-type errors, and exact-arity errors.

### Recovery evaluation

The semantic milestone was completed before probing incomplete arrows. Two existing Oxc recovery
paths are trustworthy for this slice and have dedicated conformance cases:

- a missing arrow return type is represented by `MissingType`, suppresses the incomplete signature,
  and preserves diagnostics in later declarations;
- an arrow block body missing its EOF `}` uses the existing function-body closer metadata, retains
  the real body, and preserves earlier independent diagnostics.

For both recovery fixtures, the syntax diagnostic matches the pinned TypeScript-Go CLI. The CLI
stops before reporting the independent assignment error, while editor-mode `tsrs` intentionally
continues; the pre-existing missing-return-type and function-body-closer fixtures have the same
differential result. These cases therefore validate the established editor-recovery policy rather
than claiming ordinary CLI diagnostic-set equality.

A concise arrow missing its expression body at a semicolon (`=> ;`) still reaches Oxc's generic
fatal unexpected-token path and cannot preserve the following declaration. Supporting that state
requires a separately designed Oxc recovery owner for the concise-body slot and a
`MissingExpression`; the checker must not invent a callable body in its absence. This parser change
is deferred rather than folded into the semantic milestone.
