// @reference bootstrap/explicitly-annotated-callable-expressions/valid

const echo = (value: string): string => value;

const count = (value: number): number => {
    const captured: number = value;
    return captured;
};

const enabled = function (value: boolean): boolean {
    return value;
};

const named = function identity(value: string): string {
    return value;
};

const echoed: string = echo("tsrs");
const counted: number = count(1);
const isEnabled: boolean = enabled(true);
const namedResult: string = named("named");
