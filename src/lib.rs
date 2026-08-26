//! An experimental TypeScript type checker built on Oxc's syntax infrastructure.
//!
//! The public API deliberately returns owned data. Oxc's arena-backed AST stays
//! inside [`check_source`], which leaves room for a future incremental `Program`
//! API without making arena lifetimes part of this bootstrap API.

mod checker;
mod diagnostic;
mod relations;
pub mod types;

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;

pub use diagnostic::{CheckResult, Diagnostic, Phase, TextRange};

/// Parse, bind, and type-check one TypeScript source file.
///
/// Type checking currently covers only explicitly annotated variable
/// declarations initialized with primitive literals. Parser and binder
/// diagnostics are complete to the extent provided by Oxc.
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
    let parsed = Parser::new(&allocator, source_text, source_type).parse();
    let mut result = CheckResult::default();
    result.extend_oxc(parsed.diagnostics, Phase::Parse);

    if parsed.panicked {
        return result;
    }

    let semantic = SemanticBuilder::new_compiler().build(&parsed.program);
    result.extend_oxc(semantic.diagnostics, Phase::Bind);

    if result.is_ok() {
        result.diagnostics.extend(checker::check(&parsed.program));
    }

    result
}
