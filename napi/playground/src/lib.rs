use napi_derive::napi;
use typescript_rs::{Diagnostic, Phase, TextRange, check_source};

#[derive(Clone, Debug, Eq, PartialEq)]
#[napi(object)]
pub struct PlaygroundTextRange {
    pub start: u32,
    pub end: u32,
}

impl From<TextRange> for PlaygroundTextRange {
    fn from(range: TextRange) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[napi(object)]
pub struct PlaygroundDiagnostic {
    pub code: String,
    pub message: String,
    pub phase: String,
    pub range: Option<PlaygroundTextRange>,
}

impl From<Diagnostic> for PlaygroundDiagnostic {
    fn from(diagnostic: Diagnostic) -> Self {
        Self {
            code: diagnostic.code,
            message: diagnostic.message,
            phase: match diagnostic.phase {
                Phase::Parse => "parse",
                Phase::Bind => "bind",
                Phase::Check => "check",
            }
            .to_owned(),
            range: diagnostic.range.map(Into::into),
        }
    }
}

fn playground_diagnostics(file_name: &str, source_text: &str) -> Vec<PlaygroundDiagnostic> {
    check_source(file_name, source_text)
        .diagnostics
        .into_iter()
        .map(Into::into)
        .collect()
}

#[napi(js_name = "checkSource")]
#[must_use]
#[allow(
    clippy::needless_pass_by_value,
    reason = "the NAPI boundary owns strings received from JavaScript"
)]
pub fn check_source_for_playground(
    file_name: String,
    source_text: String,
) -> Vec<PlaygroundDiagnostic> {
    playground_diagnostics(&file_name, &source_text)
}

#[cfg(test)]
mod tests {
    use typescript_rs::check_source;

    use super::playground_diagnostics;

    #[test]
    fn browser_diagnostics_are_an_exact_owned_projection() {
        let source = "const value: number = \"wrong\";";
        let native = check_source("example.ts", source);
        let browser = playground_diagnostics("example.ts", source);

        assert_eq!(browser.len(), native.diagnostics.len());
        for (browser, native) in browser.iter().zip(native.diagnostics) {
            assert_eq!(browser.code, native.code);
            assert_eq!(browser.message, native.message);
            assert_eq!(browser.phase, native.phase.to_string());
            assert_eq!(
                browser.range.as_ref().map(|range| (range.start, range.end)),
                native.range.map(|range| (range.start, range.end))
            );
        }
    }
}
