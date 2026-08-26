// @reference bootstrap/annotated-callable-foundations/argument-errors

function format(value: string, count: number): string {
    return value;
}

function setStatus(status: "open" | "closed"): "open" | "closed" {
    return status;
}

format(42, 1);
format("ok", "once");
setStatus("pending");

