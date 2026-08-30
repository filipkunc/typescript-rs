//! An experimental TypeScript type checker built on Oxc's syntax infrastructure.
//!
//! The public API deliberately returns owned data. Oxc's arena-backed AST stays
//! inside [`check_source`], which leaves room for a future incremental `Program`
//! API without making arena lifetimes part of this bootstrap API.

mod checker;
mod diagnostic;
mod relations;
mod signatures;
pub mod types;

use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::Program};
use oxc_ast_visit::Visit;
use oxc_parser::{ParseMode, ParseOptions, Parser};
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

pub use diagnostic::{CheckResult, Diagnostic, Phase, TextRange};

/// Parse, bind, and type-check one TypeScript source file.
///
/// Type checking currently covers explicitly annotated variable declarations, simple identifier
/// assignments, property-only interfaces, and the narrow annotated callable declaration and
/// expression milestones documented in the repository README. Parser and binder diagnostics are
/// complete to the extent provided by Oxc, with a narrow compatibility normalization for common
/// unfinished input.
#[must_use]
pub fn check_source(file_name: &str, source_text: &str) -> CheckResult {
    let Ok(source_type) = SourceType::from_path(file_name) else {
        return CheckResult::from_diagnostic(Diagnostic::new(
            "TSRS0001",
            format!("unsupported source file extension for '{file_name}'"),
            Phase::Parse,
            None,
        ));
    };

    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source_text, source_type)
        .with_options(ParseOptions {
            mode: ParseMode::Editor,
            ..ParseOptions::default()
        })
        .parse();
    let has_parser_recovery = !parsed.recoveries.is_empty();
    let mut result = CheckResult::default();
    result.extend_parse_diagnostics(parsed.diagnostics, source_text);

    if parsed.panicked {
        return result;
    }

    let contains_recovery =
        !result.is_ok() && (has_parser_recovery || contains_editor_recovery(&parsed.program));
    let semantic = SemanticBuilder::new_compiler().build(&parsed.program);
    let can_check = semantic.diagnostics.is_empty() && (result.is_ok() || contains_recovery);
    result.extend_oxc(semantic.diagnostics, Phase::Bind);

    if can_check {
        result.diagnostics.extend(checker::check(
            &parsed.program,
            semantic.semantic.scoping(),
            &parsed.recoveries,
        ));
    }

    result
}

#[derive(Default)]
struct RecoveryFinder {
    found: bool,
}

impl<'a> Visit<'a> for RecoveryFinder {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        self.found |= matches!(
            kind,
            AstKind::MissingExpression(_)
                | AstKind::MalformedExpression(_)
                | AstKind::MissingMemberExpression(_)
                | AstKind::MissingType(_)
        );
    }
}

fn contains_editor_recovery<'a>(program: &'a Program<'a>) -> bool {
    let mut finder = RecoveryFinder::default();
    finder.visit_program(program);
    finder.found
}
