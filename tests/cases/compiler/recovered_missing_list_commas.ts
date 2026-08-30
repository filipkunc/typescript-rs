// @reference bootstrap/editor-recovery/missing-list-commas
type Shape = { first: number; second: number };
const object: Shape = {
  first: 1
  second: "wrong",
};
const array: number[] = [
  1
  "wrong",
];
function check(first: number, second: number): void {}
check(
  1
  "wrong",
);
const intact: number = "also wrong";
