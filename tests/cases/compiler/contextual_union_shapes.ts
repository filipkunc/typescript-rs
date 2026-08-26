// @reference: bootstrap/contextual-union-shapes
type Success = { kind: "success"; values: number[] };
type Failure = { kind: "failure"; message: string };
type Result = Success | Failure;
type LetterArrays = "a"[] | "b"[];

const success: Result = { kind: "success", values: [1, -2] };
const failure: Result = { kind: "failure", message: "nope" };
const letters: LetterArrays = ["a", "a"];
const invalidKind: Result = { kind: "other", values: [1] };
