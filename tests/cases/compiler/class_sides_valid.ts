// @reference bootstrap/classes/instance-constructor-static-sides

class Box {
    value: number = 1;
    static label: string = "Box";

    constructor(seed: number) {}

    read(prefix: string): number {
        return this.value;
    }

    static describe(seed: number): string {
        return "Box";
    }
}

function identity(box: Box): Box {
    return box;
}

const box: Box = new Box(1);
const value: number = box.value;
const read: number = box.read("prefix");
const label: string = Box.label;
const description: string = Box.describe(2);
const identical: Box = identity(box);
