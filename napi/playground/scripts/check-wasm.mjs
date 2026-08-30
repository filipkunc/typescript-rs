import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { checkSource } = require("../tsrs.wasi.cjs");

const source = 'const value: number = "wrong";';
const diagnostics = checkSource("example.ts", source);

assert.deepEqual(diagnostics, [
  {
    code: "TS2322",
    message: "Type 'string' is not assignable to type 'number'.",
    phase: "check",
    range: { start: 6, end: 11 },
  },
]);

console.log("tsrs WASM diagnostics match the native check_source projection");
