// @reference bootstrap/editor-recovery/missing-object-property-value
type Shape = { missing: number; wrong: number };
const value: Shape = {
  missing: ,
  wrong: "wrong",
};
const intact: number = "also wrong";
