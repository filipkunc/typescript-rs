// @reference bootstrap/editor-recovery/missing-type-annotation
type Shape = {
  unchecked: ;
  wrong: number;
};
const value: Shape = {
  unchecked: { anything: true },
  wrong: "wrong",
};
const missing: = "ignored";
type Broken = ;
type BrokenUnion = string | ;
const intact: number = "also wrong";
