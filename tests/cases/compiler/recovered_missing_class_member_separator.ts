// @reference bootstrap/editor-recovery/missing-class-member-separator

class Box { first: number = 1 second: string = "ok"; }

const box: Box = new Box();
const first: number = box.first;
const wrong: number = "wrong";
