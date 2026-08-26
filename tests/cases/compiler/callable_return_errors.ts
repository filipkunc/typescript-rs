// @reference bootstrap/annotated-callable-foundations/return-errors

function wrongPrimitive(value: string): number {
    return value;
}

function wrongLiteral(): "ok" {
    return "not-ok";
}

function wrongArray(values: string[]): number[] {
    return values;
}

