# Oxc active-context resynchronization slice

Status: implemented and validated for the first bounded slice.

## Problem

Editor mode can already synthesize a missing initializer when the token after `=` is a local
delimiter such as `;`. It still aborts when the next token belongs to a following statement:

```ts
const broken =
const intact: number = "wrong";
```

The second `const` is not an expression operand. It is a trustworthy element starter owned by the
active source-element list. Consuming it, or failing fatally before returning control to that list,
loses the intact declaration and its type diagnostic.

## TypeScript-Go reference model

TypeScript-Go keeps a bitset of active parsing contexts. During error recovery it asks whether the
current token is an element or terminator of any active context. If so, the inner parser returns a
missing node without consuming the token, leaving the owning outer parser to resume. Otherwise it
consumes a token and continues searching. Element-start predicates may be stricter in recovery
mode than during ordinary parsing.

This slice adopts that ownership rule without copying TypeScript-Go's unified AST. Oxc editor mode
continues to represent the absent initializer as a zero-width `MissingExpression`.

## Shared recovery operations

Recovery contexts expose three conceptual operations:

1. `is_recovery_element_start(context)` identifies a token that can begin a trustworthy element
   of that context while recovering.
2. `is_recovery_terminator(context)` identifies a token that ends that context.
3. `at_recovery_context_boundary()` asks those questions across all active contexts.

The first implementation intentionally fills only the predicates needed by this grammar slice.
Source and block statement contexts recognize the unambiguous variable-statement starters
`const` and `var`. Variable-declaration context recognizes its comma-separated declarators and
statement terminators. Existing expression-list delimiter recovery remains on its current helper
until a later focused migration.

## First slice

While parsing a statement variable-declaration list:

- activate a `VariableDeclarations` recovery context;
- after consuming an initializer `=`, ask the shared active-context query whether the current
  token belongs to any active context;
- in editor mode, emit `Expression expected.`, create a zero-width `MissingExpression`, and leave
  the boundary token unconsumed;
- allow the statement parser to resume at an unambiguous following `const` or `var`, even when no
  newline permits ordinary automatic semicolon insertion.

Both forms must preserve the following declaration:

```ts
const broken =
const intact: number = "wrong";

const alsoBroken = const alsoIntact: number = "wrong";
```

Normal parse mode remains unchanged and may reject both inputs fatally.

## Invariants

- A boundary token is never consumed by the inner missing-expression recovery.
- The synthesized node has an empty span at that token's start.
- Each recovery either returns to an owning context or consumes input; statement-list progress
  checks remain valid.
- The recovered declaration is not given an invented initializer type.
- Independent following declarations still bind and type-check.
- Existing valid-source parsing and allocation behavior remain unchanged outside editor mode.

## Explicit exclusions

This is not general panic-mode recovery. It does not add contextual parsing for ambiguous `let`,
arbitrary statement starters, nested declaration forms, unfinished strings, regular expressions,
or templates. It does not change lexer resynchronization. Those areas require their own pinned
TypeScript-Go probes and focused fixtures.

## Conformance and validation

Add separate named conformance cases for newline and same-line declaration boundaries. Each must
assert both the parse diagnostic at the missing initializer and the surviving assignment-type
diagnostic in the following declaration. Add Oxc parser tests for AST preservation and normal-mode
isolation, plus an LSP test covering the user's edit state. Compare the syntax boundary with the
pinned TypeScript-Go checkout, then run the root and affected Oxc release gates and the checker
benchmark.

The pinned TypeScript-Go parser probe confirms exact parity for both layouts: `TS1109` begins at
the following declaration token, two variable statements survive, and both `broken` and `intact`
are bound. The offline recovery manifest now contains 27 cases and passes exact Oxc comparison.

The full compiler differential has one intentional policy difference. TypeScript-Go reports only
the syntax error for a file containing parse errors, while `tsrs` editor mode continues checking
trustworthy recovered syntax and additionally reports the later `TS2322`. Both new cases match
TypeScript-Go's syntax diagnostic exactly; the extra type diagnostic is the editor-recovery goal,
not a parser-parity failure.
