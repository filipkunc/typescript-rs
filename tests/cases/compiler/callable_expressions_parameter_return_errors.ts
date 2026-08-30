// @reference bootstrap/explicitly-annotated-callable-expressions/parameter-return-errors

const concise = (value: string): number => value;

const arrowBlock = (value: number): string => {
    return value;
};

const ordinary = function (value: boolean): string {
    return value;
};
