# Oxc fork compatibility

`tsrs` carries the Oxc recovery prototype as the `vendor/oxc` Git submodule. Cargo resolves all
direct Oxc dependencies from that checkout; Oxc's own workspace path dependencies keep its AST,
parser, visitors, diagnostics, and semantic crates on one source revision.

## Current baseline

- Published Oxc release previously used by `tsrs`: `0.147.0`
- Upstream release revision (`crates_v0.147.0`): `4e258430cdb290598d9f2aeb2d13be598ec9e8e9`
- Fork: <https://github.com/filipkunc/oxc>
- Pinned fork revision: `4e258430cdb290598d9f2aeb2d13be598ec9e8e9`
- Fork `main` observed at adoption: `33ac4b0915e66b2908953e85340ad59556449c05`
  (71 commits ahead of the release revision)
- Playground fork: <https://github.com/filipkunc/playground>
- Pinned playground revision: `357b37b4602f6e56423bc17383acd9bc5677f1c3`
- Required `tsrs` integration changes: Cargo path dependencies, submodule-aware checkout, and no
  checker API changes

The submodule deliberately starts at the exact release revision rather than bundling the 71
unrelated post-release commits from fork `main`. It declares the Oxc crates consumed by `tsrs` as
version `0.147.0`. This pin is the pre-recovery baseline; it does not yet contain a `tsrs`-specific
recovery AST change.

## Updating the pin

Develop recovery changes on a dedicated branch in the fork. Before recording a newer gitlink:

1. run the focused parser, AST visitor, and semantic tests in `vendor/oxc`;
2. run the complete `tsrs` formatting, linting, test, and rustdoc checks;
3. update this note with the new fork revision and any required `tsrs` API changes;
4. review both the Oxc commit range and the superproject gitlink change.

Fresh checkouts must initialize the submodule with `git submodule update --init --recursive`.
Run `./scripts/oxc-playground setup` once and `./scripts/oxc-playground serve` to inspect the pinned
Oxc revision in Monaco. The frontend's sibling dependency resolves to `vendor/oxc/napi/playground`.
