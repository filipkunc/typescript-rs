use criterion::{Criterion, criterion_group, criterion_main};
use typescript_rs::check_source;

const SOURCE: &str = r#"
export const title: string = "tsrs";
export const version: number = 1;
export const enabled: boolean = true;

export function repeat(value: string, count: number): string[] {
    return Array.from({ length: count }, () => value);
}
"#;

const LITERAL_UNION_SOURCE: &str = r#"
type Status = "open" | "closed";
type Result = Status | "failed";

export const open: Status = "open";
export const failed: Result = "failed";
export const invalid: Status = "pending";
"#;

const OBJECT_SHAPE_SOURCE: &str = r#"
type User = { id: number; name: string; active?: boolean };

export const ada: User = { id: 1, name: "Ada", active: true };
export const grace: User = { id: 2, name: "Grace" };
export const invalid: User = { id: "3", name: "Lin" };
"#;

const ARRAY_SHAPE_SOURCE: &str = r#"
type User = { id: number; name: string; tags: string[] };
type Groups = User[][];

export const groups: Groups = [
    [{ id: 1, name: "Ada", tags: ["compiler", "math"] }],
    [{ id: 2, name: "Grace", tags: ["cobol"] }],
];
export const invalid: Groups = [[{ id: 3, name: "Lin", tags: [false] }]];
"#;

const GENERIC_ARRAY_SHAPE_SOURCE: &str = r#"
type User = { id: number; name: string; tags: Array<string> };
type Groups = Array<Array<User>>;

export const groups: Groups = [
    [{ id: 1, name: "Ada", tags: ["compiler", "math"] }],
    [{ id: 2, name: "Grace", tags: ["cobol"] }],
];
export const invalid: Groups = [[{ id: 3, name: "Lin", tags: [false] }]];
"#;

const STRUCTURAL_DIAGNOSTIC_SOURCE: &str = r#"
type Server = { host: string; ports: number[] };
type Config = { servers: Server[][] };

export const config: Config = {
    servers: [[
        { host: "localhost", ports: [80, 443] },
        { host: 42, ports: [3000, "invalid"], secure: true },
        { ports: [8080] },
    ]],
};
"#;

const ANNOTATED_CALLABLE_SOURCE: &str = r#"
type Status = "open" | "closed";

function selectStatus(status: Status): Status {
    return status;
}

function echo(value: string): string {
    return value;
}

export const selected: Status = selectStatus("open");
export const message: string = echo("tsrs");
selectStatus("pending");
echo(42);
"#;

const PROPERTY_INTERFACE_SOURCE: &str = r#"
type Status = "active" | "disabled";

interface Address {
    city: string;
    coordinates: number[];
}

interface User {
    id: number;
    status: Status;
    address: Address;
}

function identity(user: User): User {
    return user;
}

export const selected: User = identity({
    id: 1,
    status: "active",
    address: { city: "Vienna", coordinates: [48.2, 16.37] },
});
"#;

const CLASS_SIDES_SOURCE: &str = r#"
class Box {
    value: number = 1;
    static label: string = "Box";
    constructor(seed: number) {}
    read(prefix: string): number { return this.value; }
    static describe(seed: number): string { return "Box"; }
}
const box: Box = new Box(1);
const value: number = box.value;
const read: number = box.read("prefix");
const label: string = Box.label;
const description: string = Box.describe(2);
"#;

const SIMPLE_ASSIGNMENT_SOURCE: &str = r#"
let inferred = "tsrs";
inferred = "rust";
inferred = 1;

let annotated: number;
annotated = 1;
annotated = "invalid";
"#;

const RECOVERED_MISSING_INITIALIZER_SOURCE: &str = r#"
const broken: number = ;
const intact: number = "wrong";
"#;

const RECOVERED_MISSING_ASSIGNMENT_RHS_SOURCE: &str = r#"
let target: number = 1;
target = ;
const intact: number = "wrong";
"#;

const RECOVERED_MISSING_OBJECT_PROPERTY_VALUE_SOURCE: &str = r#"
type Shape = { missing: number; wrong: number };
const value: Shape = { missing: , wrong: "wrong" };
const intact: number = "also wrong";
"#;

const RECOVERED_MISSING_ARRAY_OPERAND_SOURCE: &str = r#"
let target: number = 1;
const values: number[] = [target = , ... , "wrong"];
const intact: number = "also wrong";
"#;

const RECOVERED_MISSING_CALL_ARGUMENT_SOURCE: &str = r#"
function check(first: number, second: number): void {}
check(, "wrong");
const intact: number = "also wrong";
"#;

const RECOVERED_MISSING_CALL_CLOSER_SOURCE: &str = r#"
function check(): void {}
check(
const intact: number = "wrong";
"#;

const RECOVERED_MISSING_DECLARATION_NAME_SOURCE: &str = r#"
const = 1;
const intact: number = "wrong";
"#;

const RECOVERED_MISSING_LIST_DELIMITERS_SOURCE: &str = r#"
type Shape = { first: number; second: number };
const object: Shape = { first: 1 second: "wrong" };
const array: number[] = [1 2];
function check(first: number, second: number): void {}
const calls = [check(1, 2];
const intact: number = "also wrong";
"#;

const RECOVERED_MISSING_PARAMETER_DELIMITER_SOURCE: &str = r#"
function format(value: number suffix: string): string {
    return suffix;
}
const result: string = format(1, "ok");
const intact: number = "also wrong";
"#;

const RECOVERED_MISSING_FUNCTION_BODY_CLOSER_SOURCE: &str = r#"
const intact: number = "wrong";
function f(): number {
    return 1;
"#;

const RECOVERED_MISSING_RETURN_EXPRESSION_OPERAND_SOURCE: &str = r#"
function broken(): number {
    return 1 +
}
const intact: number = "wrong";
"#;

const RECOVERED_FUNCTION_INTERFACE_EDITS_SOURCE: &str = r#"
interface Box { value: number label: string }
declare const box: Box;
box.;
box?.;
function broken(, second: number): void {}
broken("ignored", 1);
const intact: number = "wrong";
interface Unclosed { value: number;
"#;

const RECOVERED_CLASS_MEMBER_SOURCE: &str = r#"
class Box { first: number = 1 second: string = "ok"; }
const box: Box = new Box();
const first: number = box.first;
const wrong: number = "wrong";
class Unclosed { value: number = 1;
"#;

const RECOVERED_MISSING_TYPE_SOURCE: &str = r#"
type Shape = { unchecked: ; wrong: number };
const value: Shape = { unchecked: true, wrong: "wrong" };
type Values = number[;
const values: Values = ["wrong"];
const intact: number = "also wrong";
"#;

const RECOVERED_MALFORMED_EXPRESSION_SOURCE: &str = r#"
let target: number = 1;
target = ...;
const broken: number = :;
const intact: number = "also wrong";
"#;

fn check_file(criterion: &mut Criterion) {
    criterion.bench_function("parse_bind_check/small_file", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", SOURCE));
    });
    criterion.bench_function("parse_bind_check/literal_unions", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", LITERAL_UNION_SOURCE));
    });
    criterion.bench_function("parse_bind_check/object_shapes", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", OBJECT_SHAPE_SOURCE));
    });
    criterion.bench_function("parse_bind_check/array_shapes", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", ARRAY_SHAPE_SOURCE));
    });
    criterion.bench_function("parse_bind_check/generic_array_shapes", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", GENERIC_ARRAY_SHAPE_SOURCE));
    });
    criterion.bench_function("parse_bind_check/structural_diagnostics", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", STRUCTURAL_DIAGNOSTIC_SOURCE));
    });
    criterion.bench_function("parse_bind_check/annotated_callables", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", ANNOTATED_CALLABLE_SOURCE));
    });
    criterion.bench_function("parse_bind_check/property_interfaces", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", PROPERTY_INTERFACE_SOURCE));
    });
    criterion.bench_function("parse_bind_check/simple_assignments", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", SIMPLE_ASSIGNMENT_SOURCE));
    });
    class_benchmarks(criterion);
    editor_recovery_benchmarks(criterion);
}

fn class_benchmarks(criterion: &mut Criterion) {
    criterion.bench_function("parse_bind_check/class_sides", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", CLASS_SIDES_SOURCE));
    });
    criterion.bench_function("parse_bind_check/editor_recovery_class_member", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", RECOVERED_CLASS_MEMBER_SOURCE));
    });
}

fn editor_recovery_benchmarks(criterion: &mut Criterion) {
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_initializer",
        |bencher| {
            bencher.iter(|| check_source("benchmark.ts", RECOVERED_MISSING_INITIALIZER_SOURCE));
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_assignment_rhs",
        |bencher| {
            bencher.iter(|| check_source("benchmark.ts", RECOVERED_MISSING_ASSIGNMENT_RHS_SOURCE));
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_object_property_value",
        |bencher| {
            bencher.iter(|| {
                check_source(
                    "benchmark.ts",
                    RECOVERED_MISSING_OBJECT_PROPERTY_VALUE_SOURCE,
                )
            });
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_array_operand",
        |bencher| {
            bencher.iter(|| check_source("benchmark.ts", RECOVERED_MISSING_ARRAY_OPERAND_SOURCE));
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_call_argument",
        |bencher| {
            bencher.iter(|| check_source("benchmark.ts", RECOVERED_MISSING_CALL_ARGUMENT_SOURCE));
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_call_closer",
        |bencher| {
            bencher.iter(|| check_source("benchmark.ts", RECOVERED_MISSING_CALL_CLOSER_SOURCE));
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_declaration_name",
        |bencher| {
            bencher
                .iter(|| check_source("benchmark.ts", RECOVERED_MISSING_DECLARATION_NAME_SOURCE));
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_list_delimiters",
        |bencher| {
            bencher.iter(|| check_source("benchmark.ts", RECOVERED_MISSING_LIST_DELIMITERS_SOURCE));
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_parameter_delimiter",
        |bencher| {
            bencher.iter(|| {
                check_source("benchmark.ts", RECOVERED_MISSING_PARAMETER_DELIMITER_SOURCE)
            });
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_function_body_closer",
        |bencher| {
            bencher.iter(|| {
                check_source(
                    "benchmark.ts",
                    RECOVERED_MISSING_FUNCTION_BODY_CLOSER_SOURCE,
                )
            });
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_missing_return_expression_operand",
        |bencher| {
            bencher.iter(|| {
                check_source(
                    "benchmark.ts",
                    RECOVERED_MISSING_RETURN_EXPRESSION_OPERAND_SOURCE,
                )
            });
        },
    );
    criterion.bench_function(
        "parse_bind_check/editor_recovery_function_interface_edits",
        |bencher| {
            bencher
                .iter(|| check_source("benchmark.ts", RECOVERED_FUNCTION_INTERFACE_EDITS_SOURCE));
        },
    );
    criterion.bench_function("parse_bind_check/editor_recovery_missing_type", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", RECOVERED_MISSING_TYPE_SOURCE));
    });
    criterion.bench_function(
        "parse_bind_check/editor_recovery_malformed_expression",
        |bencher| {
            bencher.iter(|| check_source("benchmark.ts", RECOVERED_MALFORMED_EXPRESSION_SOURCE));
        },
    );
}

criterion_group!(benches, check_file);
criterion_main!(benches);
