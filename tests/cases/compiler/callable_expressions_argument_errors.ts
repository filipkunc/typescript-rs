// @reference bootstrap/explicitly-annotated-callable-expressions/argument-errors

const format = (value: string, count: number): string => value;

const setStatus = function (status: "open" | "closed"): "open" | "closed" {
    return status;
};

format(42, 1);
format("ok", "once");
setStatus("pending");
