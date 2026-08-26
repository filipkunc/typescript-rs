// @reference bootstrap/annotated-callable-foundations/symbol-resolution

function identity(value: string): string {
    return value;
}

function forward(value: string): string {
    return identity(value);
}

const beforeDeclaration: number = declaredLater(1);

function declaredLater(value: number): number {
    return value;
}

const forwarded: string = forward("resolved");

