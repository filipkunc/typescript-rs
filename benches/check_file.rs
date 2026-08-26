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

fn check_file(criterion: &mut Criterion) {
    criterion.bench_function("parse_bind_check/small_file", |bencher| {
        bencher.iter(|| check_source("benchmark.ts", SOURCE));
    });
}

criterion_group!(benches, check_file);
criterion_main!(benches);
