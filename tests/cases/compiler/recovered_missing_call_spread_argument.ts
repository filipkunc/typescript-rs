// @reference bootstrap/editor-recovery/missing-call-spread-argument
function check(first: number, second: number): void {}
check(... , "wrong");
const intact: number = "also wrong";
