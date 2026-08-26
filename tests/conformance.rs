use std::{env, fs, path::Path};

use typescript_rs::check_source;

const REGISTERED_CASES: &[&str] = &["primitive_literals.ts", "valid_primitive_literals.ts"];

// Keep these as ordinary, explicitly named Rust tests. rust-analyzer can then
// expose each TypeScript fixture as its own item in VS Code's Test Explorer.
#[test]
fn primitive_literals() {
    run_case("primitive_literals.ts");
}

#[test]
fn valid_primitive_literals() {
    run_case("valid_primitive_literals.ts");
}

#[test]
fn every_fixture_has_a_registered_test() {
    let root = cases_root();
    let mut discovered: Vec<_> = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ts"))
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    discovered.sort();

    assert_eq!(
        discovered, REGISTERED_CASES,
        "add a named #[test] and REGISTERED_CASES entry for every fixture so it appears in Test Explorer"
    );
}

fn run_case(relative_path: &str) {
    let case = cases_root().join(relative_path);
    let source = fs::read_to_string(&case)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", case.display()));
    let actual = check_source(&case.to_string_lossy(), &source).render_concise(&source);
    let baseline = case.with_extension("errors");

    if env::var_os("BLESS").is_some() {
        fs::write(&baseline, &actual)
            .unwrap_or_else(|error| panic!("failed to update {}: {error}", baseline.display()));
        return;
    }

    let expected = fs::read_to_string(&baseline).unwrap_or_default();
    if actual != expected {
        let actual_path = case.with_extension("actual");
        fs::write(&actual_path, &actual)
            .unwrap_or_else(|error| panic!("failed to write {}: {error}", actual_path.display()));
        panic!(
            "{}\nexpected:\n{}\nactual:\n{}\nfull output: {}",
            case.display(),
            expected,
            actual,
            actual_path.display()
        );
    }
}

fn cases_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases/compiler")
}
