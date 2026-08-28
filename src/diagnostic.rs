use std::fmt::{self, Display, Write};

use oxc_diagnostics::Diagnostics;

/// The compiler stage that produced a diagnostic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Parse,
    Bind,
    Check,
}

impl Display for Phase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse => formatter.write_str("parse"),
            Self::Bind => formatter.write_str("bind"),
            Self::Check => formatter.write_str("check"),
        }
    }
}

/// A byte range into the UTF-8 source text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TextRange {
    pub start: u32,
    pub end: u32,
}

impl TextRange {
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }
}

/// An owned diagnostic suitable for a CLI, test baseline, or future LSP layer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub phase: Phase,
    pub range: Option<TextRange>,
}

impl Diagnostic {
    pub(crate) fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        phase: Phase,
        range: Option<TextRange>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            phase,
            range,
        }
    }
}

/// The result of checking a single source file.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CheckResult {
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    pub(crate) fn from_diagnostic(diagnostic: Diagnostic) -> Self {
        Self {
            diagnostics: vec![diagnostic],
        }
    }

    pub(crate) fn extend_oxc(&mut self, diagnostics: Diagnostics, phase: Phase) {
        self.extend_owned_oxc(diagnostics, phase, None);
    }

    pub(crate) fn extend_parse_diagnostics(&mut self, diagnostics: Diagnostics, source_text: &str) {
        self.extend_owned_oxc(diagnostics, Phase::Parse, Some(source_text));
    }

    fn extend_owned_oxc(
        &mut self,
        diagnostics: Diagnostics,
        phase: Phase,
        source_text: Option<&str>,
    ) {
        for diagnostic in diagnostics {
            let code = if diagnostic.code.is_some() {
                diagnostic.code.to_string()
            } else {
                match phase {
                    Phase::Parse => "TSRS1000".to_owned(),
                    Phase::Bind => "TSRS1001".to_owned(),
                    Phase::Check => "TSRS1002".to_owned(),
                }
            };
            let label = diagnostic
                .labels
                .iter()
                .find(|label| label.primary())
                .or_else(|| diagnostic.labels.first());
            let range = label.map(|label| {
                TextRange::new(label.offset(), label.offset().saturating_add(label.len()))
            });
            let message = diagnostic.message.to_string();
            if let Some(source_text) = source_text
                && message == "Unexpected token"
                && let Some(normalized) = normalize_unexpected_token(source_text, range)
            {
                self.diagnostics
                    .extend(normalized.into_iter().map(|diagnostic| {
                        Diagnostic::new(
                            diagnostic.code,
                            diagnostic.message,
                            diagnostic.phase,
                            Some(diagnostic.range),
                        )
                    }));
                continue;
            }
            self.diagnostics
                .push(Diagnostic::new(code, message, phase, range));
        }
    }

    /// Returns `true` when no diagnostics were produced.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }

    /// Render deterministic one-line diagnostics for conformance baselines.
    #[must_use]
    pub fn render_concise(&self, source_text: &str) -> String {
        let mut output = String::new();
        for diagnostic in &self.diagnostics {
            if let Some(range) = diagnostic.range {
                let (line, column) = line_and_column(source_text, range.start);
                let _ = writeln!(
                    output,
                    "{}@{}:{} [{}] {}",
                    diagnostic.code, line, column, diagnostic.phase, diagnostic.message
                );
            } else {
                let _ = writeln!(
                    output,
                    "{} [{}] {}",
                    diagnostic.code, diagnostic.phase, diagnostic.message
                );
            }
        }
        output
    }
}

struct NormalizedParseDiagnostic {
    code: &'static str,
    message: &'static str,
    phase: Phase,
    range: TextRange,
}

fn normalize_unexpected_token(
    source_text: &str,
    range: Option<TextRange>,
) -> Option<Vec<NormalizedParseDiagnostic>> {
    let range = range?;
    let trimmed = source_text.trim_end();
    let trimmed_end = u32::try_from(trimmed.len()).ok()?;
    if range.end < trimmed_end {
        return None;
    }

    if let Some(keyword_range) = trailing_keyword_range(source_text, trimmed_end, "let") {
        return Some(vec![
            NormalizedParseDiagnostic {
                code: "TS1212",
                message: "Identifier expected. 'let' is a reserved word in strict mode.",
                phase: Phase::Parse,
                range: keyword_range,
            },
            NormalizedParseDiagnostic {
                code: "TS2304",
                message: "Cannot find name 'let'.",
                phase: Phase::Check,
                range: keyword_range,
            },
        ]);
    }
    if trimmed.ends_with(";.") {
        return Some(vec![NormalizedParseDiagnostic {
            code: "TS1128",
            message: "Declaration or statement expected.",
            phase: Phase::Parse,
            range,
        }]);
    }
    if trimmed.ends_with('=') && !ends_with_type_alias_declaration(trimmed) {
        return Some(vec![NormalizedParseDiagnostic {
            code: "TS1109",
            message: "Expression expected.",
            phase: Phase::Parse,
            range: TextRange::new(trimmed_end, trimmed_end),
        }]);
    }
    None
}

fn trailing_keyword_range(source_text: &str, trimmed_end: u32, keyword: &str) -> Option<TextRange> {
    let trimmed_end = usize::try_from(trimmed_end).ok()?;
    let line_start = source_text[..trimmed_end]
        .rfind(['\n', '\r'])
        .map_or(0, |index| index + 1);
    let line = &source_text[line_start..trimmed_end];
    let leading_whitespace = line.len() - line.trim_start().len();
    (line.trim_start() == keyword).then(|| {
        let start = line_start + leading_whitespace;
        TextRange::new(
            u32::try_from(start).expect("source offset was already representable as u32"),
            u32::try_from(start + keyword.len())
                .expect("source offset was already representable as u32"),
        )
    })
}

fn ends_with_type_alias_declaration(source_text: &str) -> bool {
    let mut words = source_text
        .rsplit_once(['\n', '\r'])
        .map_or(source_text, |(_, line)| line)
        .split_whitespace();
    let mut word = words.next();
    while matches!(word, Some("export" | "declare")) {
        word = words.next();
    }
    word == Some("type")
}

fn line_and_column(source_text: &str, byte_offset: u32) -> (usize, usize) {
    let offset = usize::try_from(byte_offset)
        .unwrap_or(usize::MAX)
        .min(source_text.len());
    let prefix = &source_text[..floor_char_boundary(source_text, offset)];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix
        .rsplit_once('\n')
        .map_or(prefix.chars().count(), |(_, tail)| tail.chars().count())
        + 1;
    (line, column)
}

fn floor_char_boundary(text: &str, mut index: usize) -> usize {
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

#[cfg(test)]
mod tests {
    use super::{TextRange, line_and_column, normalize_unexpected_token};

    #[test]
    fn calculates_unicode_locations() {
        assert_eq!(line_and_column("const π = 1;\nπ", 14), (2, 1));
    }

    #[test]
    fn does_not_treat_an_incomplete_type_alias_as_an_expression() {
        let source = "type Value =";
        let end = u32::try_from(source.len()).expect("test source fits in a text range");

        assert!(normalize_unexpected_token(source, Some(TextRange::new(end, end))).is_none());
    }
}
