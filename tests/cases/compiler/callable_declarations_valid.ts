// @reference bootstrap/annotated-callable-foundations/declarations-valid

export function identity(value: string): string {
    return value;
}

function count(value: number): number {
    return value;
}

const message: string = identity("tsrs");
const total: number = count(3);
