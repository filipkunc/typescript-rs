// @reference bootstrap/editor-recovery/missing-call-assignment-right-hand-side
function check(first: number, second: number): void {}
let target: number = 1;
check(target = , "wrong");
const intact: number = "also wrong";
