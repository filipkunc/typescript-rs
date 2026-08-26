use oxc_ast::ast::{Expression, Program, TSType, VariableDeclarator};
use oxc_ast_visit::{Visit, walk::walk_variable_declarator};
use oxc_span::GetSpan;

use crate::{
    Diagnostic, Phase, TextRange,
    types::{TypeId, TypeKind, TypeStore},
};

pub(crate) fn check(program: &Program<'_>) -> Vec<Diagnostic> {
    let mut checker = Checker::default();
    checker.visit_program(program);
    checker.diagnostics
}

#[derive(Default)]
struct Checker {
    diagnostics: Vec<Diagnostic>,
    types: TypeStore,
}

impl<'a> Visit<'a> for Checker {
    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        if let (Some(annotation), Some(initializer)) =
            (&declaration.type_annotation, &declaration.init)
            && let (Some(expected), Some(actual)) = (
                self.type_from_annotation(&annotation.type_annotation),
                self.type_from_expression(initializer),
            )
            && !self.is_assignable(actual, expected)
        {
            let span = initializer.span();
            let Some(actual) = self.types.kind(actual) else {
                unreachable!("checker produced a TypeId outside its own store")
            };
            let Some(expected) = self.types.kind(expected) else {
                unreachable!("checker produced a TypeId outside its own store")
            };
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

impl Checker {
    fn type_from_annotation(&self, annotation: &TSType<'_>) -> Option<TypeId> {
        let primitives = self.types.primitives();
        match annotation {
            TSType::TSAnyKeyword(_) => Some(primitives.any),
            TSType::TSUnknownKeyword(_) => Some(primitives.unknown),
            TSType::TSNeverKeyword(_) => Some(primitives.never),
            TSType::TSVoidKeyword(_) => Some(primitives.void),
            TSType::TSUndefinedKeyword(_) => Some(primitives.undefined),
            TSType::TSNullKeyword(_) => Some(primitives.null),
            TSType::TSBooleanKeyword(_) => Some(primitives.boolean),
            TSType::TSNumberKeyword(_) => Some(primitives.number),
            TSType::TSBigIntKeyword(_) => Some(primitives.bigint),
            TSType::TSStringKeyword(_) => Some(primitives.string),
            TSType::TSParenthesizedType(parenthesized) => {
                self.type_from_annotation(&parenthesized.type_annotation)
            }
            _ => None,
        }
    }

    fn type_from_expression(&self, expression: &Expression<'_>) -> Option<TypeId> {
        let primitives = self.types.primitives();
        match expression {
            Expression::BooleanLiteral(_) => Some(primitives.boolean),
            Expression::BigIntLiteral(_) => Some(primitives.bigint),
            Expression::NullLiteral(_) => Some(primitives.null),
            Expression::NumericLiteral(_) => Some(primitives.number),
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => {
                Some(primitives.string)
            }
            Expression::Identifier(identifier) if identifier.name == "undefined" => {
                Some(primitives.undefined)
            }
            Expression::ParenthesizedExpression(parenthesized) => {
                self.type_from_expression(&parenthesized.expression)
            }
            _ => None,
        }
    }

    fn is_assignable(&self, source: TypeId, target: TypeId) -> bool {
        source == target
            || matches!(
                self.types.kind(target),
                Some(TypeKind::Any | TypeKind::Unknown)
            )
    }
}
