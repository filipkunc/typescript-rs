// @reference bootstrap/editor-recovery/missing-interface-member-type
interface Shape {
  unchecked: ;
  wrong: number;
}
const value: Shape = { unchecked: true, wrong: "wrong" };
const intact: number = "also wrong";
