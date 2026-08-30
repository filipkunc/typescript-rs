// @reference bootstrap/editor-recovery/missing-parameter
function broken(, second: number): void {}
broken("ignored", "wrong");
const intact: number = "also wrong";
