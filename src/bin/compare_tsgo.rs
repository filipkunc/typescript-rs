use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Output},
};

use typescript_rs::{Diagnostic, check_source};

const TSGO_REPOSITORY_ENV: &str = "TSGO_REPO";
const REFERENCE_REVISION_FILE: &str = "tests/tsgo-reference.txt";

#[derive(Debug)]
struct Options {
    repository: PathBuf,
    case: Option<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ComparableDiagnostic {
    code: String,
    location: Option<(usize, usize)>,
    message: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<bool, String> {
    let Some(options) = parse_options()? else {
        return Ok(true);
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let expected_revision = read_reference_revision(&root.join(REFERENCE_REVISION_FILE))?;
    let repository = canonicalize(&options.repository, "TypeScript-Go repository")?;
    verify_revision(&repository, &expected_revision)?;
    let tsgo = tsgo_binary(&repository);
    if !tsgo.is_file() {
        return Err(format!(
            "{} does not exist; build the pinned checkout with `npm ci` followed by `npx hereby local`",
            tsgo.display()
        ));
    }

    let cases_root = root.join("tests/cases/compiler");
    let mut cases = discover_cases(&cases_root)?;
    if let Some(case) = options.case {
        cases.retain(|path| case_matches(path, &case));
        if cases.is_empty() {
            return Err(format!("no conformance fixture matches '{case}'"));
        }
    }

    let mut mismatches = 0_usize;
    for case in &cases {
        let source = fs::read_to_string(case)
            .map_err(|error| format!("failed to read {}: {error}", case.display()))?;
        let file_name = case.to_string_lossy();
        let tsrs =
            comparable_tsrs_diagnostics(&source, &check_source(&file_name, &source).diagnostics);
        let tsgo = comparable_tsgo_diagnostics(&run_tsgo(&tsgo, &repository, case)?)?;
        let relative = case.strip_prefix(root).unwrap_or(case);
        if tsrs == tsgo {
            println!("ok   {}", relative.display());
        } else {
            mismatches += 1;
            println!("FAIL {}", relative.display());
            print_difference("tsrs", &tsrs);
            print_difference("tsgo", &tsgo);
        }
    }

    if mismatches == 0 {
        println!(
            "all {} fixture(s) match TypeScript-Go {expected_revision}",
            cases.len()
        );
        Ok(true)
    } else {
        eprintln!(
            "{mismatches} of {} fixture(s) differ from TypeScript-Go {expected_revision}",
            cases.len()
        );
        Ok(false)
    }
}

fn parse_options() -> Result<Option<Options>, String> {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let mut repository = env::var_os(TSGO_REPOSITORY_ENV).map(PathBuf::from);
    let mut case = None;

    while let Some(argument) = arguments.next() {
        match argument.to_str() {
            Some("--repo") => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--repo requires a path".to_owned())?;
                repository = Some(PathBuf::from(value));
            }
            Some("--case") => {
                let value = arguments
                    .next()
                    .ok_or_else(|| "--case requires a fixture name".to_owned())?;
                case = Some(
                    value
                        .into_string()
                        .map_err(|_| "--case must be valid UTF-8".to_owned())?,
                );
            }
            Some("--help" | "-h") => {
                print_usage();
                return Ok(None);
            }
            Some(other) => return Err(format!("unknown argument '{other}'")),
            None => return Err("arguments must be valid UTF-8".to_owned()),
        }
    }

    let repository = repository.ok_or_else(|| {
        format!("pass --repo <typescript-go-checkout> or set {TSGO_REPOSITORY_ENV}")
    })?;
    Ok(Some(Options { repository, case }))
}

fn print_usage() {
    println!(
        "Usage: cargo test-tsgo --repo <typescript-go-checkout> [--case <fixture>]\n\
         \n\
         The checkout must be at the revision pinned in {REFERENCE_REVISION_FILE} and built with:\n\
           npm ci\n\
           npx hereby local\n\
         \n\
         {TSGO_REPOSITORY_ENV} may be used instead of --repo."
    );
}

fn read_reference_revision(path: &Path) -> Result<String, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let revisions: Vec<_> = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    let [revision] = revisions.as_slice() else {
        return Err(format!(
            "{} must contain exactly one reference revision",
            path.display()
        ));
    };
    if revision.len() != 40 || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("invalid reference revision '{revision}'"));
    }
    Ok((*revision).to_owned())
}

fn canonicalize(path: &Path, description: &str) -> Result<PathBuf, String> {
    path.canonicalize().map_err(|error| {
        format!(
            "failed to resolve {description} {}: {error}",
            path.display()
        )
    })
}

fn verify_revision(repository: &Path, expected: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repository)
        .output()
        .map_err(|error| format!("failed to inspect {}: {error}", repository.display()))?;
    if !output.status.success() {
        return Err(format!(
            "{} is not a readable Git checkout: {}",
            repository.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let actual = String::from_utf8(output.stdout)
        .map_err(|_| "git rev-parse returned non-UTF-8 output".to_owned())?;
    let actual = actual.trim();
    if actual != expected {
        return Err(format!(
            "TypeScript-Go revision mismatch: expected {expected}, found {actual}; reference updates require reviewing and editing {REFERENCE_REVISION_FILE}"
        ));
    }
    Ok(())
}

fn tsgo_binary(repository: &Path) -> PathBuf {
    repository
        .join("built/local")
        .join(format!("tsgo{}", env::consts::EXE_SUFFIX))
}

fn discover_cases(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut cases: Vec<_> = fs::read_dir(root)
        .map_err(|error| format!("failed to read {}: {error}", root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ts"))
        .collect();
    cases.sort();
    Ok(cases)
}

fn case_matches(path: &Path, requested: &str) -> bool {
    path.file_name().is_some_and(|name| {
        name == requested || path.file_stem().is_some_and(|stem| stem == requested)
    })
}

fn run_tsgo(binary: &Path, repository: &Path, case: &Path) -> Result<Output, String> {
    let output = Command::new(binary)
        .args([
            "--strict",
            "--target",
            "esnext",
            "--module",
            "esnext",
            "--lib",
            "esnext",
            "--noEmit",
            "--pretty",
            "false",
            "--noErrorTruncation",
            "--skipLibCheck",
        ])
        .arg(case)
        .current_dir(repository)
        .output()
        .map_err(|error| format!("failed to run {}: {error}", binary.display()))?;
    if output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty() {
        Ok(output)
    } else {
        Err(format!(
            "{} exited with {} and produced no diagnostics",
            binary.display(),
            output.status
        ))
    }
}

fn comparable_tsrs_diagnostics(
    source: &str,
    diagnostics: &[Diagnostic],
) -> Vec<ComparableDiagnostic> {
    let mut comparable: Vec<_> = diagnostics
        .iter()
        .map(|diagnostic| ComparableDiagnostic {
            code: diagnostic.code.clone(),
            location: diagnostic
                .range
                .map(|range| utf16_line_and_column(source, range.start)),
            message: normalize_message(&diagnostic.message),
        })
        .collect();
    comparable.sort();
    comparable
}

fn comparable_tsgo_diagnostics(output: &Output) -> Result<Vec<ComparableDiagnostic>, String> {
    let stdout = String::from_utf8(output.stdout.clone())
        .map_err(|_| "TypeScript-Go stdout was not valid UTF-8".to_owned())?;
    let stderr = String::from_utf8(output.stderr.clone())
        .map_err(|_| "TypeScript-Go stderr was not valid UTF-8".to_owned())?;
    let text = match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => String::new(),
    };
    let mut diagnostics = parse_tsgo_diagnostics(&text)?;
    if output.status.success() && !diagnostics.is_empty() {
        return Err("TypeScript-Go succeeded after reporting diagnostics".to_owned());
    }
    if !output.status.success() && diagnostics.is_empty() {
        return Err(format!(
            "TypeScript-Go exited with {} without parseable diagnostics",
            output.status
        ));
    }
    diagnostics.sort();
    Ok(diagnostics)
}

fn parse_tsgo_diagnostics(text: &str) -> Result<Vec<ComparableDiagnostic>, String> {
    let mut diagnostics: Vec<ComparableDiagnostic> = Vec::new();
    for line in text.lines().map(str::trim_end) {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(diagnostic) = parse_tsgo_diagnostic_header(line)? {
            diagnostics.push(diagnostic);
        } else if let Some(diagnostic) = diagnostics.last_mut() {
            diagnostic.message = normalize_message(&format!("{} {line}", diagnostic.message));
        } else {
            return Err(format!("unrecognized TypeScript-Go output: {line}"));
        }
    }
    Ok(diagnostics)
}

fn parse_tsgo_diagnostic_header(line: &str) -> Result<Option<ComparableDiagnostic>, String> {
    const LOCATED_MARKER: &str = "): error TS";
    const GLOBAL_MARKER: &str = "error TS";

    if let Some(marker) = line.rfind(LOCATED_MARKER) {
        let prefix = &line[..marker];
        let open = prefix
            .rfind('(')
            .ok_or_else(|| format!("missing diagnostic location in: {line}"))?;
        let (line_number, column) = parse_location(&prefix[open + 1..])?;
        let (code, message) = parse_code_and_message(&line[marker + LOCATED_MARKER.len()..])?;
        return Ok(Some(ComparableDiagnostic {
            code,
            location: Some((line_number, column)),
            message,
        }));
    }
    if let Some(rest) = line.strip_prefix(GLOBAL_MARKER) {
        let (code, message) = parse_code_and_message(rest)?;
        return Ok(Some(ComparableDiagnostic {
            code,
            location: None,
            message,
        }));
    }
    Ok(None)
}

fn parse_location(text: &str) -> Result<(usize, usize), String> {
    let (line, column) = text
        .split_once(',')
        .ok_or_else(|| format!("invalid diagnostic location '{text}'"))?;
    let line = line
        .parse()
        .map_err(|_| format!("invalid diagnostic line '{line}'"))?;
    let column = column
        .parse()
        .map_err(|_| format!("invalid diagnostic column '{column}'"))?;
    Ok((line, column))
}

fn parse_code_and_message(text: &str) -> Result<(String, String), String> {
    let (code, message) = text
        .trim_start()
        .split_once(':')
        .ok_or_else(|| format!("invalid diagnostic text '{text}'"))?;
    if code.is_empty() || !code.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("invalid diagnostic code '{code}'"));
    }
    Ok((format!("TS{code}"), normalize_message(message)))
}

fn normalize_message(message: &str) -> String {
    message.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn utf16_line_and_column(source: &str, offset: u32) -> (usize, usize) {
    let mut offset = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix, |(_, tail)| tail)
        .encode_utf16()
        .count()
        + 1;
    (line, column)
}

fn print_difference(engine: &str, diagnostics: &[ComparableDiagnostic]) {
    println!("  {engine}:");
    if diagnostics.is_empty() {
        println!("    <no diagnostics>");
    } else {
        for diagnostic in diagnostics {
            match diagnostic.location {
                Some((line, column)) => println!(
                    "    {}@{line}:{column} {}",
                    diagnostic.code, diagnostic.message
                ),
                None => println!("    {} {}", diagnostic.code, diagnostic.message),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ComparableDiagnostic, normalize_message, parse_tsgo_diagnostics, utf16_line_and_column,
    };

    #[test]
    fn parses_located_global_and_continued_diagnostics() {
        let diagnostics = parse_tsgo_diagnostics(
            "/tmp/case.ts(2,7): error TS2322: Type 'number' is not assignable\n  to type 'string'.\nerror TS5093: Global problem",
        )
        .expect("diagnostics should parse");

        assert_eq!(
            diagnostics,
            vec![
                ComparableDiagnostic {
                    code: "TS2322".to_owned(),
                    location: Some((2, 7)),
                    message: "Type 'number' is not assignable to type 'string'.".to_owned(),
                },
                ComparableDiagnostic {
                    code: "TS5093".to_owned(),
                    location: None,
                    message: "Global problem".to_owned(),
                },
            ]
        );
    }

    #[test]
    fn counts_utf16_columns() {
        assert_eq!(utf16_line_and_column("😀x", 4), (1, 3));
        assert_eq!(utf16_line_and_column("first\n😀x", 10), (2, 3));
    }

    #[test]
    fn normalizes_diagnostic_message_whitespace() {
        assert_eq!(normalize_message("one\n  two\tthree"), "one two three");
    }
}
