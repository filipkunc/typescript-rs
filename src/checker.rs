use std::collections::{HashMap, HashSet};

use oxc_ast::ast::{
    ArrayExpression, Expression, ObjectExpression, ObjectPropertyKind, Program, Statement,
    TSLiteral, TSSignature, TSType, TSTypeName, UnaryOperator, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk::walk_variable_declarator};
use oxc_span::GetSpan;

use crate::{
    Diagnostic, Phase, TextRange,
    relations::TypeRelations,
    types::{NumberLiteral, ObjectTypeProperty, TypeId, TypeKind, TypeStore},
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
        if self.needs_structural_diagnostics(initializer, expected) {
            let expected_text = self.annotation_text(&annotation.type_annotation).to_owned();
            if let Some(diagnostics) =
                self.structural_diagnostics(initializer, expected, Some(&expected_text))
            {
                if !diagnostics.is_empty() {
                    self.diagnostics.extend(diagnostics);
                    return;
                }
                return;
            }
        }

        let Some(actual) = self.type_from_expression(initializer, Some(expected)) else {
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
            TSType::TSArrayType(array) => {
                let element = self.type_from_annotation(&array.element_type)?;
                Some(self.types.array(element))
            }
            TSType::TSTypeLiteral(literal) => {
                let properties: Option<Vec<_>> = literal
                    .members
                    .iter()
                    .map(|member| {
                        let TSSignature::TSPropertySignature(property) = member else {
                            return None;
                        };
                        if property.computed {
                            return None;
                        }
                        let name = property.key.static_name()?.into_owned();
                        let annotation = property.type_annotation.as_ref()?;
                        Some(ObjectTypeProperty {
                            name,
                            type_id: self.type_from_annotation(&annotation.type_annotation)?,
                            optional: property.optional,
                        })
                    })
                    .collect();
                Some(self.types.object(properties?))
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
            TSLiteral::UnaryExpression(unary) if unary.operator == UnaryOperator::UnaryNegation => {
                let Expression::NumericLiteral(literal) = &unary.argument else {
                    return None;
                };
                TypeKind::NumberLiteral(NumberLiteral::new(-literal.value))
            }
            TSLiteral::TemplateLiteral(_) | TSLiteral::UnaryExpression(_) => return None,
        };
        Some(self.types.intern(kind))
    }

    fn type_from_expression(
        &mut self,
        expression: &Expression<'a>,
        contextual_target: Option<TypeId>,
    ) -> Option<TypeId> {
        let primitives = self.types.primitives();
        let preserve_literals =
            contextual_target.is_some_and(|target| self.requires_literal_identity(target));
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
            Expression::UnaryExpression(unary)
                if unary.operator == UnaryOperator::UnaryNegation =>
            {
                let Expression::NumericLiteral(literal) = &unary.argument else {
                    return None;
                };
                if preserve_literals {
                    Some(
                        self.types
                            .intern(TypeKind::NumberLiteral(NumberLiteral::new(-literal.value))),
                    )
                } else {
                    Some(primitives.number)
                }
            }
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
            Expression::ArrayExpression(array) => {
                let element_target = contextual_target
                    .and_then(|target| self.contextual_array_element_target(target));
                let elements: Option<Vec<_>> = array
                    .elements
                    .iter()
                    .map(|element| {
                        self.type_from_expression(element.as_expression()?, element_target)
                    })
                    .collect();
                let element = self.types.union(elements?);
                Some(self.types.array(element))
            }
            Expression::ObjectExpression(object) => {
                let properties: Option<Vec<_>> = object
                    .properties
                    .iter()
                    .map(|member| {
                        let ObjectPropertyKind::ObjectProperty(property) = member else {
                            return None;
                        };
                        if property.computed || property.method {
                            return None;
                        }
                        let name = property.key.static_name()?.into_owned();
                        let property_target = contextual_target.and_then(|target| {
                            self.contextual_object_property_target(target, &name)
                        });
                        Some(ObjectTypeProperty {
                            name,
                            type_id: self.type_from_expression(&property.value, property_target)?,
                            optional: false,
                        })
                    })
                    .collect();
                Some(self.types.object(properties?))
            }
            Expression::ParenthesizedExpression(parenthesized) => {
                self.type_from_expression(&parenthesized.expression, contextual_target)
            }
            _ => None,
        }
    }

    fn contextual_array_element_target(&mut self, target: TypeId) -> Option<TypeId> {
        let elements = match self.types.kind(target)? {
            TypeKind::Array(element) => return Some(*element),
            TypeKind::Union(members) => members
                .iter()
                .filter_map(|member| match self.types.kind(*member) {
                    Some(TypeKind::Array(element)) => Some(*element),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => return None,
        };
        (!elements.is_empty()).then(|| self.types.union(elements))
    }

    fn contextual_object_property_target(&mut self, target: TypeId, name: &str) -> Option<TypeId> {
        let property_types = match self.types.kind(target)? {
            TypeKind::Object(properties) => {
                return properties
                    .iter()
                    .find(|property| property.name == name)
                    .map(|property| property.type_id);
            }
            TypeKind::Union(members) => members
                .iter()
                .filter_map(|member| {
                    let Some(TypeKind::Object(properties)) = self.types.kind(*member) else {
                        return None;
                    };
                    properties
                        .iter()
                        .find(|property| property.name == name)
                        .map(|property| property.type_id)
                })
                .collect::<Vec<_>>(),
            _ => return None,
        };
        (!property_types.is_empty()).then(|| self.types.union(property_types))
    }

    fn needs_structural_diagnostics(&self, expression: &Expression<'a>, target: TypeId) -> bool {
        match expression {
            Expression::ObjectExpression(_) => self.target_contains_object(target),
            Expression::ArrayExpression(_) => self.target_contains_array(target),
            Expression::ParenthesizedExpression(parenthesized) => {
                self.needs_structural_diagnostics(&parenthesized.expression, target)
            }
            _ => false,
        }
    }

    fn target_contains_object(&self, target: TypeId) -> bool {
        match self.types.kind(target) {
            Some(TypeKind::Object(_)) => true,
            Some(TypeKind::Union(members)) => members
                .iter()
                .any(|member| matches!(self.types.kind(*member), Some(TypeKind::Object(_)))),
            _ => false,
        }
    }

    fn target_contains_array(&self, target: TypeId) -> bool {
        match self.types.kind(target) {
            Some(TypeKind::Array(_)) => true,
            Some(TypeKind::Union(members)) => members
                .iter()
                .any(|member| matches!(self.types.kind(*member), Some(TypeKind::Array(_)))),
            _ => false,
        }
    }

    fn structural_diagnostics(
        &mut self,
        expression: &Expression<'a>,
        target: TypeId,
        target_label: Option<&str>,
    ) -> Option<Vec<Diagnostic>> {
        if let Expression::ParenthesizedExpression(parenthesized) = expression {
            return self.structural_diagnostics(&parenthesized.expression, target, target_label);
        }

        if let Some(TypeKind::Union(members)) = self.types.kind(target) {
            let members = members.to_vec();
            return self.union_structural_diagnostics(expression, target, &members, target_label);
        }

        match (expression, self.types.kind(target)) {
            (Expression::ObjectExpression(object), Some(TypeKind::Object(properties))) => {
                let properties = properties.to_vec();
                self.object_structural_diagnostics(
                    expression,
                    object,
                    target,
                    &properties,
                    target_label,
                )
            }
            (Expression::ArrayExpression(array), Some(TypeKind::Array(element))) => {
                let element = *element;
                self.array_structural_diagnostics(array, element)
            }
            _ => Some(
                self.type_mismatch_diagnostic(expression, target, target_label)
                    .into_iter()
                    .collect(),
            ),
        }
    }

    fn union_structural_diagnostics(
        &mut self,
        expression: &Expression<'a>,
        target: TypeId,
        members: &[TypeId],
        target_label: Option<&str>,
    ) -> Option<Vec<Diagnostic>> {
        let candidates: Vec<_> = match expression {
            Expression::ObjectExpression(_) => members
                .iter()
                .copied()
                .filter(|member| matches!(self.types.kind(*member), Some(TypeKind::Object(_))))
                .collect(),
            Expression::ArrayExpression(_) => members
                .iter()
                .copied()
                .filter(|member| matches!(self.types.kind(*member), Some(TypeKind::Array(_))))
                .collect(),
            _ => Vec::new(),
        };
        let mut best = None;
        for candidate in candidates {
            let Some(diagnostics) = self.structural_diagnostics(expression, candidate, None) else {
                continue;
            };
            if diagnostics.is_empty() {
                return Some(diagnostics);
            }
            if best
                .as_ref()
                .is_none_or(|current: &Vec<Diagnostic>| diagnostics.len() < current.len())
            {
                best = Some(diagnostics);
            }
        }
        best.or_else(|| {
            Some(
                self.type_mismatch_diagnostic(expression, target, target_label)
                    .into_iter()
                    .collect(),
            )
        })
    }

    fn object_structural_diagnostics(
        &mut self,
        expression: &Expression<'a>,
        object: &ObjectExpression<'a>,
        target: TypeId,
        properties: &[ObjectTypeProperty],
        target_label: Option<&str>,
    ) -> Option<Vec<Diagnostic>> {
        if object.properties.iter().any(|member| {
            !matches!(member, ObjectPropertyKind::ObjectProperty(property) if !property.computed && !property.method)
        }) {
            return None;
        }
        let mut diagnostics = Vec::new();
        let has_missing = properties
            .iter()
            .filter(|property| !property.optional)
            .any(|property| {
                !object.properties.iter().any(|member| {
                    let ObjectPropertyKind::ObjectProperty(source) = member else {
                        return false;
                    };
                    source
                        .key
                        .static_name()
                        .is_some_and(|name| name == property.name)
                })
            });
        if has_missing {
            let actual = self.type_from_expression(expression, Some(target))?;
            let actual = self.types.display(actual).to_string();
            let expected = self.target_text(target, target_label);
            for property in properties.iter().filter(|property| {
                !property.optional
                    && !object.properties.iter().any(|member| {
                        let ObjectPropertyKind::ObjectProperty(source) = member else {
                            return false;
                        };
                        source
                            .key
                            .static_name()
                            .is_some_and(|name| name == property.name)
                    })
            }) {
                diagnostics.push(Diagnostic::new(
                    "TS2741",
                    format!(
                        "Property '{}' is missing in type '{actual}' but required in type '{expected}'.",
                        property.name
                    ),
                    Phase::Check,
                    Some(Self::range(object.span)),
                ));
            }
        }

        for member in &object.properties {
            let ObjectPropertyKind::ObjectProperty(source) = member else {
                return None;
            };
            let name = source.key.static_name()?.into_owned();
            if let Some(property) = properties.iter().find(|property| property.name == name) {
                diagnostics.extend(self.structural_diagnostics(
                    &source.value,
                    property.type_id,
                    None,
                )?);
            } else {
                let expected = self.target_text(target, target_label);
                diagnostics.push(Diagnostic::new(
                    "TS2353",
                    format!(
                        "Object literal may only specify known properties, and '{name}' does not exist in type '{expected}'."
                    ),
                    Phase::Check,
                    Some(Self::range(source.key.span())),
                ));
            }
        }
        Some(diagnostics)
    }

    fn array_structural_diagnostics(
        &mut self,
        array: &ArrayExpression<'a>,
        element: TypeId,
    ) -> Option<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        for item in &array.elements {
            diagnostics.extend(self.structural_diagnostics(
                item.as_expression()?,
                element,
                None,
            )?);
        }
        Some(diagnostics)
    }

    fn type_mismatch_diagnostic(
        &mut self,
        expression: &Expression<'a>,
        target: TypeId,
        target_label: Option<&str>,
    ) -> Option<Diagnostic> {
        let actual = self.type_from_expression(expression, Some(target))?;
        if self.relations.is_assignable(&self.types, actual, target) {
            return None;
        }
        let actual = self.diagnostic_source_type(actual, target);
        let actual = self.types.display(actual);
        let expected = self.target_text(target, target_label);
        Some(Diagnostic::new(
            "TS2322",
            format!("Type '{actual}' is not assignable to type '{expected}'."),
            Phase::Check,
            Some(Self::range(expression.span())),
        ))
    }

    fn target_text(&self, target: TypeId, target_label: Option<&str>) -> String {
        target_label.map_or_else(|| self.types.display(target).to_string(), str::to_owned)
    }

    const fn range(span: oxc_span::Span) -> TextRange {
        TextRange::new(span.start, span.end)
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
