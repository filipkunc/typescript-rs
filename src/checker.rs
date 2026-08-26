use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    Expression, Program, Statement, TSLiteral, TSType, TSTypeName, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk::walk_variable_declarator};
use oxc_span::GetSpan;

use crate::{
    Diagnostic, Phase, TextRange,
    relations::TypeRelations,
    types::{NumberLiteral, TypeId, TypeKind, TypeStore},
};

pub(crate) fn check<'a>(program: &'a Program<'a>) -> Vec<Diagnostic> {
    let aliases = program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::TSTypeAliasDeclaration(alias) if alias.type_parameters.is_none() => {
                Some((alias.id.name.as_str(), &alias.type_annotation))
            }
            _ => None,
        })
        .collect();
    let mut checker = Checker::new(program.source_text, aliases);
    checker.visit_program(program);
    checker.diagnostics
}

struct Checker<'a> {
    source_text: &'a str,
    aliases: HashMap<&'a str, &'a TSType<'a>>,
    resolving_aliases: HashSet<&'a str>,
    diagnostics: Vec<Diagnostic>,
    types: TypeStore,
    relations: TypeRelations,
}

impl<'a> Visit<'a> for Checker<'a> {
    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        self.check_variable_declarator(declaration);

        walk_variable_declarator(self, declaration);
    }
}

impl<'a> Checker<'a> {
    fn new(source_text: &'a str, aliases: HashMap<&'a str, &'a TSType<'a>>) -> Self {
        Self {
            source_text,
            aliases,
            resolving_aliases: HashSet::new(),
            diagnostics: Vec::new(),
            types: TypeStore::new(),
            relations: TypeRelations::default(),
        }
    }

    fn check_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        let (Some(annotation), Some(initializer)) =
            (&declaration.type_annotation, &declaration.init)
        else {
            return;
        };
        let Some(expected) = self.type_from_annotation(&annotation.type_annotation) else {
            return;
        };
        let preserve_literals = self.requires_literal_identity(expected);
        let Some(actual) = self.type_from_expression(initializer, preserve_literals) else {
            return;
        };
        if self.relations.is_assignable(&self.types, actual, expected) {
            return;
        }

        let span = initializer.span();
        let actual = self.diagnostic_source_type(actual, expected);
        let actual = self.types.display(actual).to_string();
        let expected = self.annotation_text(&annotation.type_annotation).to_owned();
        self.diagnostics.push(Diagnostic::new(
            "TS2322",
            format!("Type '{actual}' is not assignable to type '{expected}'."),
            Phase::Check,
            Some(TextRange::new(span.start, span.end)),
        ));
    }

    fn type_from_annotation(&mut self, annotation: &TSType<'a>) -> Option<TypeId> {
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
            TSType::TSLiteralType(literal) => self.type_from_literal(&literal.literal),
            TSType::TSUnionType(union) => {
                let members: Option<Vec<_>> = union
                    .types
                    .iter()
                    .map(|member| self.type_from_annotation(member))
                    .collect();
                Some(self.types.union(members?))
            }
            TSType::TSTypeReference(reference) if reference.type_arguments.is_none() => {
                let TSTypeName::IdentifierReference(name) = &reference.type_name else {
                    return None;
                };
                let name = name.name.as_str();
                let alias = self.aliases.get(name).copied()?;
                if !self.resolving_aliases.insert(name) {
                    return None;
                }
                let resolved = self.type_from_annotation(alias);
                self.resolving_aliases.remove(name);
                resolved
            }
            TSType::TSParenthesizedType(parenthesized) => {
                self.type_from_annotation(&parenthesized.type_annotation)
            }
            _ => None,
        }
    }

    fn type_from_literal(&mut self, literal: &TSLiteral<'a>) -> Option<TypeId> {
        let kind = match literal {
            TSLiteral::BooleanLiteral(literal) => TypeKind::BooleanLiteral(literal.value),
            TSLiteral::NumericLiteral(literal) => {
                TypeKind::NumberLiteral(NumberLiteral::new(literal.value))
            }
            TSLiteral::BigIntLiteral(literal) => TypeKind::BigIntLiteral(literal.value.to_string()),
            TSLiteral::StringLiteral(literal) => TypeKind::StringLiteral(literal.value.to_string()),
            TSLiteral::TemplateLiteral(_) | TSLiteral::UnaryExpression(_) => return None,
        };
        Some(self.types.intern(kind))
    }

    fn type_from_expression(
        &mut self,
        expression: &Expression<'a>,
        preserve_literals: bool,
    ) -> Option<TypeId> {
        let primitives = self.types.primitives();
        match expression {
            Expression::BooleanLiteral(literal) if preserve_literals => {
                Some(self.types.intern(TypeKind::BooleanLiteral(literal.value)))
            }
            Expression::BooleanLiteral(_) => Some(primitives.boolean),
            Expression::BigIntLiteral(literal) if preserve_literals => Some(
                self.types
                    .intern(TypeKind::BigIntLiteral(literal.value.to_string())),
            ),
            Expression::BigIntLiteral(_) => Some(primitives.bigint),
            Expression::NullLiteral(_) => Some(primitives.null),
            Expression::NumericLiteral(literal) if preserve_literals => Some(
                self.types
                    .intern(TypeKind::NumberLiteral(NumberLiteral::new(literal.value))),
            ),
            Expression::NumericLiteral(_) => Some(primitives.number),
            Expression::StringLiteral(literal) if preserve_literals => Some(
                self.types
                    .intern(TypeKind::StringLiteral(literal.value.to_string())),
            ),
            Expression::StringLiteral(_) | Expression::TemplateLiteral(_) => {
                Some(primitives.string)
            }
            Expression::Identifier(identifier) if identifier.name == "undefined" => {
                Some(primitives.undefined)
            }
            Expression::ParenthesizedExpression(parenthesized) => {
                self.type_from_expression(&parenthesized.expression, preserve_literals)
            }
            _ => None,
        }
    }

    fn requires_literal_identity(&self, target: TypeId) -> bool {
        matches!(
            self.types.kind(target),
            Some(
                TypeKind::BooleanLiteral(_)
                    | TypeKind::NumberLiteral(_)
                    | TypeKind::BigIntLiteral(_)
                    | TypeKind::StringLiteral(_)
                    | TypeKind::Union(_)
            )
        )
    }

    fn diagnostic_source_type(&self, source: TypeId, target: TypeId) -> TypeId {
        if matches!(
            self.types.kind(target),
            Some(
                TypeKind::Boolean
                    | TypeKind::Number
                    | TypeKind::BigInt
                    | TypeKind::String
                    | TypeKind::Null
                    | TypeKind::Undefined
            )
        ) {
            self.types.widen_literal(source)
        } else {
            source
        }
    }

    fn annotation_text(&self, annotation: &TSType<'a>) -> &'a str {
        let span = annotation.span();
        let start = usize::try_from(span.start).expect("source span must fit in usize");
        let end = usize::try_from(span.end).expect("source span must fit in usize");
        &self.source_text[start..end]
    }
}
