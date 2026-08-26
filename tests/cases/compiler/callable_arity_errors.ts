// @reference bootstrap/annotated-callable-foundations/arity-errors

function pair(left: string, right: string): string {
    return left;
}

function nothing(): void {
    return;
}

pair("left");
pair("left", "right", "extra");
nothing(1);

