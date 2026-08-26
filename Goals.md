# TypeScript in Rust

There is currently a [TypeScript port to Go](https://github.com/microsoft/typescript-go) my main goal here is to try Rust instead.

## Bootstrap

Lets say how to approach it as minimalistic as possible:

  1. Use [oxc.rs](https://github.com/oxc-project/oxc) AST and also look at [type-aware linting](https://github.com/oxc-project/tsgolint)
  2. Research the main type checking code, the idea is to be able to handle initially type checking only and rely on existing type stripping via oxc transforms.
  3. Identical port would leave a lot of performance optimizations and also architectural decisions too limited, for this reason I usually like to approach things with spec/test case research which allows more flexibility in implementation
  4. I am sure there already were approaches for either type checking or executing TypeScript from Rust, have a look into those
  5. Stretch goal is executing too and be able to handle both TypeScript and AssemblyScript, but this is not in the initial phase

## Approach

Not everything is possible to do at once, bootstrap phase is very important, it has to be also optimized for human use in VS Code not just LLMs. If the tests cannot be easily debugged or spec/test cases are not easy to synchronize with the reference Go implementation then human reviewer has very hard time to steer the efforts.
Initially phase has to be rapid but also minimal to be able to adjust, follow-up phases are iterative. Not everything can be done perfect on the first try in port like this.
Setting up benchmarks early is vital to see it makes sense.
Build time is also very important to be able to move fast. This might be harder in Rust, but oxc.rs is already excellent we can learn from it.
