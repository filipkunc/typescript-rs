// @reference bootstrap/explicitly-annotated-callable-expressions/arity-errors

const pair = (left: string, right: string): string => left;

const nothing = function (): void {
    return;
};

pair("left");
pair("left", "right", "extra");
nothing(1);
