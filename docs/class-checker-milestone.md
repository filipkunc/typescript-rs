# Class checker milestone

Stage 4 introduces classes as a checker feature, not merely as syntax retained for editor recovery.
It first completes the object-member path shared by interfaces and class instances, then models a
class declaration with distinct instance and constructor/static sides.

## Semantic prerequisite

The existing structural object representation gains source-side property access for statically
named members. Explicitly annotated interface method signatures remain in the checker-owned
`SignatureStore` and can be selected by a static member call. Property reads produce their declared
`TypeId`; method calls reuse the existing argument, arity, and return-type operations. A missing
member reports `TS2339` at the member name. Computed, private, optional-chain, union-distributed,
and dynamically named member access remain outside this increment.

## Supported class shape

The first class slice accepts unique, top-level, non-generic class declarations with no heritage,
`implements` clause, decorators, `abstract`, or `declare` modifier. It supports:

- public/default instance and static properties with explicit annotations and optional
  initializers;
- public/default instance and static methods with simple required parameters and explicit return
  types;
- one ordinary constructor with simple required annotated parameters, or an implicit zero-argument
  constructor;
- direct `new ClassName(...)`, instance/static property reads, and instance/static method calls;
  and
- `this` property reads inside supported instance methods.

The instance side is a structural object `TypeId` made from instance properties. Instance methods
are checker-owned signatures associated with that type identity. The class value symbol instead
maps to a named constructor/static-side type identity and constructor signature. A type
reference to the class name resolves to the instance side, while `new` returns that instance type.
This makes it impossible to expose a static member through an instance, or an instance member
through the class value, without special cases in Oxc.

Property initializers, method returns, constructor/new arguments, and method-call arguments reuse
the existing assignability and structural-diagnostic operations. Method bodies remain ordinary Oxc
syntax and semantic scopes; only type identities and signatures are owned by `tsrs`.

## Exclusions

Class expressions; inheritance and `super`; `implements`; generics; overloads; accessors; static
blocks; parameter properties; private/protected members; computed names; optional/definite/
`declare`/`abstract` members; decorators; async/generator methods; constructor-body definite
assignment; instance-member assignment; self-referential or mutually class-typed members; and
callable/constructable structural relations remain future work. These exclusions keep the first
class representation small enough to revise before a program model exists.

## Recovery boundary

After the supported class semantics pass conformance and TypeScript-Go differential tests, the Oxc
fork may add bounded class-body recovery. Recovery must preserve the class identity and trustworthy
members without inventing member names or moving instance/static-side semantics into Oxc. Each new
edit state requires normal/editor parser tests, semantic safety, inspection, conformance, LSP,
manifest, benchmark, documentation, and playground evidence before it joins the supported set.
