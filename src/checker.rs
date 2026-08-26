use oxc_ast::ast::{Expression, Program, TSType, VariableDeclarator};
use oxc_ast_visit::{Visit, walk::walk_variable_declarator};
use oxc_span::GetSpan;

use crate::{Diagnostic, Phase, TextRange};

pub(crate) fn check(program: &Program<'_>) -> Vec<Diagnostic> {
    let mut checker = Checker::default();
    checker.visit_program(program);
    checker.diagnostics
}

#[derive(Default)]
struct Checker {
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Visit<'a> for Checker {
    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        if let (Some(annotation), Some(initializer)) =
            (&declaration.type_annotation, &declaration.init)
            && let (Some(expected), Some(actual)) = (
                Primitive::from_annotation(&annotation.type_annotation),
                Primitive::from_expression(initializer),
            )
            && !expected.accepts(actual)
        {
            let span = initializer.span();
            self.diagnostics.push(Diagnostic::new(
                "TS2322",
                format!("Type '{actual}' is not assignable to type '{expected}'."),
                Phase::Check,
                Some(TextRange::new(span.start, span.end)),
            ));
        }

        walk_variable_declarator(self, declaration);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Primitive {
    Any,
    Unknown,
    Boolean,
    BigInt,
    Null,
    Number,
    String,
    Undefined,
}

impl Primitive {
    fn from_annotation(annotation: &TSType<'_>) -> Option<Self> {
        match annotation {
            TSType::TSAnyKeyword(_) => Some(Self::Any),
            TSType::TSUnknownKeyword(_) => Some(Self::Unknown),
            TSType::TSBooleanKeyword(_) => Some(Self::Boolean),
            TSType::TSBigIntKeyword(_) => Some(Self::BigInt),
            TSType::TSNullKeyword(_) => Some(Self::Null),
            TSType::TSNumberKeyword(_) => Some(Self::Number),
            TSType::TSStringKeyword(_) => Some(Self::String),
            TSType::TSUndefinedKeyword(_) => Some(Self::Undefined),
            TSType::TSParenthesizedType(parenthesized) => {
                Self::from_annotation(&parenthesized.type_annotation)
            }
            _ => None,
        }
    }

    fn from_expression(expression: &Expression<'_>) -> Option<Self> {
        match expression {
            Expression::BooleanLiteral(_) => Some(Self::Boolean),
            Expression::BigIntLiteral(_) => Some(Self::BigInt),
            Expression::NullLiteral(_) => Some(Self::Null),
            Expression::NumericLiteral(_) => Some(Self::Number),
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => Some(Self::String),
            Expression::Identifier(identifier) if identifier.name == "undefined" => {
                Some(Self::Undefined)
            }
            Expression::ParenthesizedExpression(parenthesized) => {
                Self::from_expression(&parenthesized.expression)
            }
            _ => None,
        }
    }

    const fn accepts(self, actual: Self) -> bool {
        matches!(self, Self::Any | Self::Unknown) || self as u8 == actual as u8
    }
}

impl std::fmt::Display for Primitive {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Any => "any",
            Self::Unknown => "unknown",
            Self::Boolean => "boolean",
            Self::BigInt => "bigint",
            Self::Null => "null",
            Self::Number => "number",
            Self::String => "string",
            Self::Undefined => "undefined",
        };
        formatter.write_str(name)
    }
}
