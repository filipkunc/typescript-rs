// @reference bootstrap/editor-recovery/missing-list-closers
type Shape = { value: number };
const objects: Shape[] = [{ value: "object"];
const afterObject: number = "after object";
type Container = { values: number[] };
const container: Container = { values: ["array" };
const afterArray: number = "after array";
function check(value: number): void {}
const calls = [check("call"];
const afterCall: number = "after call";
