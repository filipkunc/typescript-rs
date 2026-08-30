// @reference bootstrap/classes/side-and-signature-errors

class Box {
    value: number = "wrong";
    static label: string = 1;

    constructor(seed: number) {}

    read(prefix: string): number {
        return "wrong";
    }
}

new Box();
new Box("wrong");

const box = new Box(1);
const wrongValue: string = box.value;
box.read(1);
Box.value;
box.label;
