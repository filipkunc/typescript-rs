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
        self.diagnostics
            .extend(diagnostics.into_iter().map(|diagnostic| {
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
                Diagnostic::new(code, diagnostic.message.to_string(), phase, range)
            }));
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
    use super::line_and_column;

    #[test]
    fn calculates_unicode_locations() {
        assert_eq!(line_and_column("const π = 1;\nπ", 14), (2, 1));
    }
}
