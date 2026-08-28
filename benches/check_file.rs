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
}

criterion_group!(benches, check_file);
criterion_main!(benches);
