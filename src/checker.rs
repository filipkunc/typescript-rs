use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

use oxc_ast::ast::{
    ArrayExpression, ArrowFunctionExpression, AssignmentExpression, AssignmentTarget,
    BindingPattern, CallExpression, Declaration, ExportDefaultDeclarationKind, Expression,
    Function, FunctionType, IdentifierReference, ObjectExpression, ObjectPropertyKind, Program,
    ReturnStatement, Statement, TSInterfaceDeclaration, TSLiteral, TSSignature, TSType, TSTypeName,
    UnaryOperator, VariableDeclarator,
};
use oxc_ast_visit::{
    Visit,
    walk::{
        walk_arrow_function_expression, walk_assignment_expression, walk_call_expression,
        walk_function, walk_return_statement, walk_variable_declarator,
    },
};
use oxc_semantic::{ScopeFlags, Scoping, SymbolId};
use oxc_span::GetSpan;

use crate::{
    Diagnostic, Phase, TextRange,
    relations::TypeRelations,
    signatures::{Signature, SignatureId, SignatureParameter, SignatureStore},
    types::{NumberLiteral, ObjectTypeProperty, TypeId, TypeKind, TypeStore},
};

pub(crate) fn check<'a>(program: &'a Program<'a>, scoping: &'a Scoping) -> Vec<Diagnostic> {
    let mut aliases = HashMap::new();
    let mut interfaces = HashMap::new();
    let mut functions = Vec::new();
    for statement in &program.body {
        if let Statement::TSTypeAliasDeclaration(alias) = statement
            && alias.type_parameters.is_none()
        {
            aliases.insert(alias.id.name.as_str(), &alias.type_annotation);
        }
        if let Some(interface) = top_level_interface(statement)
            && interface.type_parameters.is_none()
            && interface.extends.is_empty()
            && interface
                .id
                .symbol_id
                .get()
                .is_some_and(|symbol| scoping.symbol_redeclarations(symbol).is_empty())
        {
            interfaces.insert(interface.id.name.as_str(), interface);
        }
        if let Some(function) = top_level_function(statement) {
            functions.push(function);
        }
    }
    let mut checker = Checker::new(program.source_text, aliases, interfaces, scoping);
    for function in functions {
        if function
            .id
            .as_ref()
            .and_then(|id| id.symbol_id.get())
            .is_some_and(|symbol| scoping.symbol_redeclarations(symbol).is_empty())
        {
            checker.register_function_signature(function);
        }
    }
    checker.visit_program(program);
    checker.diagnostics
}

fn top_level_interface<'a>(statement: &'a Statement<'a>) -> Option<&'a TSInterfaceDeclaration<'a>> {
    match statement {
        Statement::TSInterfaceDeclaration(interface) => Some(interface),
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::TSInterfaceDeclaration(interface) => Some(interface),
            _ => None,
        },
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            ExportDefaultDeclarationKind::TSInterfaceDeclaration(interface) => Some(interface),
            _ => None,
        },
        _ => None,
    }
}

fn top_level_function<'a>(statement: &'a Statement<'a>) -> Option<&'a Function<'a>> {
    match statement {
        Statement::FunctionDeclaration(function) => Some(function),
        Statement::ExportDeclaration(export) => match &export.declaration {
            Declaration::FunctionDeclaration(function) => Some(function),
            _ => None,
        },
        Statement::ExportDefaultDeclaration(export) => match &export.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => Some(function),
            _ => None,
        },
        _ => None,
    }
}

struct Checker<'a> {
    source_text: &'a str,
    scoping: &'a Scoping,
    aliases: HashMap<&'a str, &'a TSType<'a>>,
    interfaces: HashMap<&'a str, &'a TSInterfaceDeclaration<'a>>,
    resolving_named_types: HashSet<&'a str>,
    diagnostics: Vec<Diagnostic>,
    types: TypeStore,
    relations: TypeRelations,
    signatures: SignatureStore,
    function_signatures: HashMap<SymbolId, SignatureId>,
    symbol_types: HashMap<SymbolId, TypeId>,
    current_signature: Option<SignatureId>,
}

impl<'a> Visit<'a> for Checker<'a> {
    fn visit_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        self.check_variable_declarator(declaration);

        walk_variable_declarator(self, declaration);
    }

    fn visit_function(&mut self, function: &Function<'a>, flags: ScopeFlags) {
        let previous = self.current_signature;
        self.current_signature = function
            .id
            .as_ref()
            .and_then(|id| id.symbol_id.get())
            .and_then(|symbol| self.function_signatures.get(&symbol).copied());
        walk_function(self, function, flags);
        self.current_signature = previous;
    }

    fn visit_return_statement(&mut self, statement: &ReturnStatement<'a>) {
        self.check_return_statement(statement);
        walk_return_statement(self, statement);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        self.check_call_expression(call);
        walk_call_expression(self, call);
    }

    fn visit_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
        self.check_assignment_expression(assignment);
        walk_assignment_expression(self, assignment);
    }

    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        let previous = self.current_signature.take();
        walk_arrow_function_expression(self, arrow);
        self.current_signature = previous;
    }
}

impl<'a> Checker<'a> {
    fn new(
        source_text: &'a str,
        aliases: HashMap<&'a str, &'a TSType<'a>>,
        interfaces: HashMap<&'a str, &'a TSInterfaceDeclaration<'a>>,
        scoping: &'a Scoping,
    ) -> Self {
        Self {
            source_text,
            scoping,
            aliases,
            interfaces,
            resolving_named_types: HashSet::new(),
            diagnostics: Vec::new(),
            types: TypeStore::new(),
            relations: TypeRelations::default(),
            signatures: SignatureStore::default(),
            function_signatures: HashMap::new(),
            symbol_types: HashMap::new(),
            current_signature: None,
        }
    }

    fn register_function_signature(&mut self, function: &Function<'a>) {
        if function.r#type != FunctionType::FunctionDeclaration
            || function.generator
            || function.r#async
            || function.declare
            || function.type_parameters.is_some()
            || function.this_param.is_some()
            || function.params.rest.is_some()
            || function.body.is_none()
        {
            return;
        }
        let (Some(id), Some(return_annotation)) = (&function.id, &function.return_type) else {
            return;
        };
        let Some(function_symbol) = id.symbol_id.get() else {
            return;
        };

        let mut parameters = Vec::with_capacity(function.params.items.len());
        let mut parameter_symbols = Vec::with_capacity(function.params.items.len());
        for parameter in &function.params.items {
            if parameter.optional
                || parameter.initializer.is_some()
                || parameter.accessibility.is_some()
                || parameter.readonly
                || parameter.r#override
                || !parameter.decorators.is_empty()
            {
                return;
            }
            let BindingPattern::BindingIdentifier(identifier) = &parameter.pattern else {
                return;
            };
            let (Some(symbol), Some(annotation)) = (
                identifier.symbol_id.get(),
                parameter.type_annotation.as_ref(),
            ) else {
                return;
            };
            let Some(type_id) = self.type_from_annotation(&annotation.type_annotation) else {
                return;
            };
            parameters.push(SignatureParameter {
                type_id,
                diagnostic_name: self.target_text(type_id, Some(&annotation.type_annotation)),
            });
            parameter_symbols.push((symbol, type_id));
        }

        let Some(return_type) = self.type_from_annotation(&return_annotation.type_annotation)
        else {
            return;
        };
        let signature = self.signatures.add(Signature {
            parameters: parameters.into_boxed_slice(),
            return_type,
            return_diagnostic_name: self
                .target_text(return_type, Some(&return_annotation.type_annotation)),
        });
        self.function_signatures.insert(function_symbol, signature);
        self.symbol_types.extend(parameter_symbols);
    }

    fn check_return_statement(&mut self, statement: &ReturnStatement<'a>) {
        let Some(signature_id) = self.current_signature else {
            return;
        };
        let Some(signature) = self.signatures.get(signature_id) else {
            return;
        };
        let target = signature.return_type;
        let expected = signature.return_diagnostic_name.clone();
        let actual = match &statement.argument {
            Some(expression) => self.type_from_expression(expression, Some(target)),
            None => Some(self.types.primitives().undefined),
        };
        let Some(actual) = actual else {
            return;
        };
        if self.relations.is_assignable(&self.types, actual, target) {
            return;
        }
        let actual = self.diagnostic_source_type(actual, target);
        let detail = self.relation_detail(actual, target);
        self.diagnostics.push(Diagnostic::new(
            "TS2322",
            format!(
                "Type '{}' is not assignable to type '{expected}'.{detail}",
                self.types.diagnostic_display(actual)
            ),
            Phase::Check,
            Some(Self::range(statement.span)),
        ));
    }

    fn check_call_expression(&mut self, call: &CallExpression<'a>) {
        let Some(signature_id) = self.signature_for_call(call) else {
            return;
        };
        let Some(signature) = self.signatures.get(signature_id) else {
            return;
        };
        let expected_count = signature.parameters.len();
        let actual_count = call.arguments.len();
        if actual_count != expected_count {
            let span = if actual_count < expected_count {
                call.callee.span()
            } else {
                call.arguments[expected_count].span()
            };
            self.diagnostics.push(Diagnostic::new(
                "TS2554",
                format!("Expected {expected_count} arguments, but got {actual_count}."),
                Phase::Check,
                Some(Self::range(span)),
            ));
            return;
        }

        for index in 0..expected_count {
            let Some(argument) = call.arguments[index].as_expression() else {
                return;
            };
            let Some(parameter) = self
                .signatures
                .get(signature_id)
                .and_then(|signature| signature.parameters.get(index))
            else {
                return;
            };
            let target = parameter.type_id;
            let expected = parameter.diagnostic_name.clone();
            let Some(actual) = self.type_from_expression(argument, Some(target)) else {
                continue;
            };
            if self.relations.is_assignable(&self.types, actual, target) {
                continue;
            }
            let actual = self.diagnostic_source_type(actual, target);
            let detail = self.relation_detail(actual, target);
            self.diagnostics.push(Diagnostic::new(
                "TS2345",
                format!(
                    "Argument of type '{}' is not assignable to parameter of type '{expected}'.{detail}",
                    self.types.diagnostic_display(actual)
                ),
                Phase::Check,
                Some(Self::range(argument.span())),
            ));
        }
    }

    fn signature_for_call(&self, call: &CallExpression<'a>) -> Option<SignatureId> {
        if call.optional || call.type_arguments.is_some() {
            return None;
        }
        let Expression::Identifier(identifier) = &call.callee else {
            return None;
        };
        let symbol = self.symbol_for_identifier(identifier)?;
        self.function_signatures.get(&symbol).copied()
    }

    fn symbol_for_identifier(&self, identifier: &IdentifierReference<'a>) -> Option<SymbolId> {
        let reference = identifier.reference_id.get()?;
        self.scoping.get_reference(reference).symbol_id()
    }

    fn relation_detail(&self, source: TypeId, target: TypeId) -> String {
        let (Some(TypeKind::Array(source)), Some(TypeKind::Array(target))) =
            (self.types.kind(source), self.types.kind(target))
        else {
            return String::new();
        };
        let source = self.diagnostic_source_type(*source, *target);
        format!(
            " Type '{}' is not assignable to type '{}'.{}",
            self.types.diagnostic_display(source),
            self.types.diagnostic_display(*target),
            self.relation_detail(source, *target)
        )
    }

    fn check_variable_declarator(&mut self, declaration: &VariableDeclarator<'a>) {
        let declared_type = declaration
            .type_annotation
            .as_ref()
            .and_then(|annotation| self.type_from_annotation(&annotation.type_annotation));

        if let (Some(annotation), Some(initializer), Some(expected)) = (
            &declaration.type_annotation,
            &declaration.init,
            declared_type,
        ) {
            self.check_variable_initializer(declaration, annotation, initializer, expected);
        }

        let variable_type = if declaration.type_annotation.is_some() {
            declared_type
        } else {
            declaration
                .init
                .as_ref()
                .and_then(|initializer| self.type_from_expression(initializer, None))
        };
        let (BindingPattern::BindingIdentifier(identifier), Some(variable_type)) =
            (&declaration.id, variable_type)
        else {
            return;
        };
        let Some(symbol) = identifier.symbol_id.get() else {
            return;
        };
        self.symbol_types.insert(symbol, variable_type);
    }

    fn check_variable_initializer(
        &mut self,
        declaration: &VariableDeclarator<'a>,
        annotation: &oxc_ast::ast::TSTypeAnnotation<'a>,
        initializer: &Expression<'a>,
        expected: TypeId,
    ) {
        if self.needs_structural_diagnostics(initializer, expected)
            && let Some(diagnostics) = self.structural_diagnostics(
                initializer,
                expected,
                Some(&annotation.type_annotation),
                Some(declaration.id.span()),
            )
        {
            self.diagnostics.extend(diagnostics);
            return;
        }

        let Some(actual) = self.type_from_expression(initializer, Some(expected)) else {
            return;
        };
        if self.relations.is_assignable(&self.types, actual, expected) {
            return;
        }

        let span = declaration.id.span();
        let actual = self.diagnostic_source_type(actual, expected);
        let actual = self.types.diagnostic_display(actual).to_string();
        let expected = self.assignment_target_text(&annotation.type_annotation, expected);
        self.diagnostics.push(Diagnostic::new(
            "TS2322",
            format!("Type '{actual}' is not assignable to type '{expected}'."),
            Phase::Check,
            Some(TextRange::new(span.start, span.end)),
        ));
    }

    fn check_assignment_expression(&mut self, assignment: &AssignmentExpression<'a>) {
        if !assignment.operator.is_assign() {
            return;
        }
        let AssignmentTarget::AssignmentTargetIdentifier(identifier) = &assignment.left else {
            return;
        };
        let Some(target) = self
            .symbol_for_identifier(identifier)
            .and_then(|symbol| self.symbol_types.get(&symbol).copied())
        else {
            return;
        };

        if self.needs_structural_diagnostics(&assignment.right, target)
            && let Some(diagnostics) =
                self.structural_diagnostics(&assignment.right, target, None, Some(identifier.span))
        {
            self.diagnostics.extend(diagnostics);
            return;
        }

        if let Some(diagnostic) =
            self.type_mismatch_diagnostic(&assignment.right, target, None, Some(identifier.span))
        {
            self.diagnostics.push(diagnostic);
        }
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
            TSType::TSTypeLiteral(literal) => self.type_from_property_members(&literal.members),
            TSType::TSTypeReference(reference) => {
                if let Some(element) = Self::generic_array_element(annotation) {
                    let element = self.type_from_annotation(element)?;
                    return Some(self.types.array(element));
                }
                if reference.type_arguments.is_some() {
                    return None;
                }
                let TSTypeName::IdentifierReference(name) = &reference.type_name else {
                    return None;
                };
                let name = name.name.as_str();
                if !self.resolving_named_types.insert(name) {
                    return None;
                }
                let resolved = if let Some(alias) = self.aliases.get(name).copied() {
                    self.type_from_annotation(alias)
                } else if let Some(interface) = self.interfaces.get(name).copied() {
                    self.type_from_interface(interface)
                } else {
                    None
                };
                self.resolving_named_types.remove(name);
                resolved
            }
            TSType::TSParenthesizedType(parenthesized) => {
                self.type_from_annotation(&parenthesized.type_annotation)
            }
            _ => None,
        }
    }

    fn type_from_interface(&mut self, interface: &'a TSInterfaceDeclaration<'a>) -> Option<TypeId> {
        if interface.type_parameters.is_some() || !interface.extends.is_empty() {
            return None;
        }
        self.type_from_property_members(&interface.body.body)
    }

    fn type_from_property_members(&mut self, members: &[TSSignature<'a>]) -> Option<TypeId> {
        let properties: Option<Vec<_>> = members
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
            Expression::Identifier(identifier) => self
                .symbol_for_identifier(identifier)
                .and_then(|symbol| self.symbol_types.get(&symbol).copied()),
            Expression::CallExpression(call) => self
                .signature_for_call(call)
                .and_then(|signature| self.signatures.get(signature))
                .map(|signature| signature.return_type),
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
        target_annotation: Option<&TSType<'a>>,
        anchor: Option<oxc_span::Span>,
    ) -> Option<Vec<Diagnostic>> {
        if let Expression::ParenthesizedExpression(parenthesized) = expression {
            return self.structural_diagnostics(
                &parenthesized.expression,
                target,
                target_annotation,
                anchor,
            );
        }

        if let Some(TypeKind::Union(members)) = self.types.kind(target) {
            let members = members.to_vec();
            return self.union_structural_diagnostics(
                expression,
                target,
                &members,
                target_annotation,
                anchor,
            );
        }

        match (expression, self.types.kind(target)) {
            (Expression::ObjectExpression(object), Some(TypeKind::Object(properties))) => {
                let properties = properties.to_vec();
                self.object_structural_diagnostics(
                    expression,
                    object,
                    target,
                    &properties,
                    target_annotation,
                    anchor,
                )
            }
            (Expression::ArrayExpression(array), Some(TypeKind::Array(element))) => {
                let element = *element;
                self.array_structural_diagnostics(array, element, target_annotation)
            }
            _ => Some(
                self.type_mismatch_diagnostic(expression, target, target_annotation, anchor)
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
        target_annotation: Option<&TSType<'a>>,
        anchor: Option<oxc_span::Span>,
    ) -> Option<Vec<Diagnostic>> {
        if let Expression::ObjectExpression(object) = expression {
            let diagnostics = self.union_object_property_diagnostics(object, target)?;
            if !diagnostics.is_empty() {
                return Some(diagnostics);
            }
        }

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
            let Some(diagnostics) =
                self.structural_diagnostics(expression, candidate, None, anchor)
            else {
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
                self.type_mismatch_diagnostic(expression, target, target_annotation, anchor)
                    .into_iter()
                    .collect(),
            )
        })
    }

    fn union_object_property_diagnostics(
        &mut self,
        object: &ObjectExpression<'a>,
        target: TypeId,
    ) -> Option<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        for member in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = member else {
                return None;
            };
            let name = property.key.static_name()?;
            let Some(property_target) = self.contextual_object_property_target(target, &name)
            else {
                continue;
            };
            diagnostics.extend(self.type_mismatch_diagnostic(
                &property.value,
                property_target,
                None,
                Some(property.key.span()),
            ));
        }
        Some(diagnostics)
    }

    fn object_structural_diagnostics(
        &mut self,
        expression: &Expression<'a>,
        object: &ObjectExpression<'a>,
        target: TypeId,
        properties: &[ObjectTypeProperty],
        target_annotation: Option<&TSType<'a>>,
        anchor: Option<oxc_span::Span>,
    ) -> Option<Vec<Diagnostic>> {
        if object.properties.iter().any(|member| {
            !matches!(member, ObjectPropertyKind::ObjectProperty(property) if !property.computed && !property.method)
        }) {
            return None;
        }
        let mut diagnostics = Vec::new();
        let missing: Vec<_> = properties
            .iter()
            .filter(|property| !property.optional)
            .filter(|property| {
                !object.properties.iter().any(|member| {
                    let ObjectPropertyKind::ObjectProperty(source) = member else {
                        return false;
                    };
                    source
                        .key
                        .static_name()
                        .is_some_and(|name| name == property.name)
                })
            })
            .collect();
        if !missing.is_empty() {
            let actual = self.type_from_expression(expression, Some(target))?;
            let actual = self.types.diagnostic_display(actual).to_string();
            let expected = self.target_text(target, target_annotation);
            if let [property] = missing.as_slice() {
                diagnostics.push(Diagnostic::new(
                    "TS2741",
                    format!(
                        "Property '{}' is missing in type '{actual}' but required in type '{expected}'.",
                        property.name
                    ),
                    Phase::Check,
                    Some(Self::range(anchor.unwrap_or(object.span))),
                ));
            } else {
                let names = missing
                    .iter()
                    .map(|property| property.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                diagnostics.push(Diagnostic::new(
                    "TS2739",
                    format!(
                        "Type '{actual}' is missing the following properties from type '{expected}': {names}"
                    ),
                    Phase::Check,
                    Some(Self::range(anchor.unwrap_or(object.span))),
                ));
            }
        }

        for member in &object.properties {
            let ObjectPropertyKind::ObjectProperty(source) = member else {
                return None;
            };
            let name = source.key.static_name()?.into_owned();
            if let Some(property) = properties.iter().find(|property| property.name == name) {
                let property_annotation = target_annotation.and_then(|annotation| {
                    self.object_property_annotation(annotation, &property.name)
                });
                diagnostics.extend(self.structural_diagnostics(
                    &source.value,
                    property.type_id,
                    property_annotation,
                    Some(source.key.span()),
                )?);
            } else {
                let expected = self.target_text(target, target_annotation);
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
        target_annotation: Option<&TSType<'a>>,
    ) -> Option<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        let element_annotation =
            target_annotation.and_then(|annotation| self.array_element_annotation(annotation));
        for item in &array.elements {
            diagnostics.extend(self.structural_diagnostics(
                item.as_expression()?,
                element,
                element_annotation,
                None,
            )?);
        }
        Some(diagnostics)
    }

    fn type_mismatch_diagnostic(
        &mut self,
        expression: &Expression<'a>,
        target: TypeId,
        target_annotation: Option<&TSType<'a>>,
        anchor: Option<oxc_span::Span>,
    ) -> Option<Diagnostic> {
        let actual = self.type_from_expression(expression, Some(target))?;
        if self.relations.is_assignable(&self.types, actual, target) {
            return None;
        }
        let actual = self.diagnostic_source_type(actual, target);
        let actual = self.types.diagnostic_display(actual);
        let expected = self.target_text(target, target_annotation);
        Some(Diagnostic::new(
            "TS2322",
            format!("Type '{actual}' is not assignable to type '{expected}'."),
            Phase::Check,
            Some(Self::range(anchor.unwrap_or_else(|| expression.span()))),
        ))
    }

    fn target_text(&self, target: TypeId, target_annotation: Option<&TSType<'a>>) -> String {
        target_annotation
            .and_then(|annotation| self.annotation_target_text(annotation, target))
            .unwrap_or_else(|| self.types.diagnostic_display(target).to_string())
    }

    fn annotation_target_text(&self, annotation: &TSType<'a>, target: TypeId) -> Option<String> {
        match annotation {
            TSType::TSTypeReference(reference) if reference.type_arguments.is_none() => {
                Some(self.annotation_text(annotation).to_owned())
            }
            TSType::TSParenthesizedType(parenthesized) => {
                self.annotation_target_text(&parenthesized.type_annotation, target)
            }
            TSType::TSTypeLiteral(literal) => {
                let Some(TypeKind::Object(properties)) = self.types.kind(target) else {
                    return None;
                };
                let mut rendered = "{ ".to_owned();
                for member in &literal.members {
                    let TSSignature::TSPropertySignature(annotation_property) = member else {
                        return None;
                    };
                    let name = annotation_property.key.static_name()?.into_owned();
                    let property = properties.iter().find(|property| property.name == name)?;
                    let property_annotation = annotation_property.type_annotation.as_ref()?;
                    let property_type = self
                        .annotation_target_text(
                            &property_annotation.type_annotation,
                            property.type_id,
                        )
                        .unwrap_or_else(|| {
                            self.types.diagnostic_display(property.type_id).to_string()
                        });
                    write!(
                        rendered,
                        "{}{}: {property_type}",
                        property.name,
                        if property.optional { "?" } else { "" }
                    )
                    .expect("writing to a string cannot fail");
                    if property.optional && !self.types.includes_undefined(property.type_id) {
                        rendered.push_str(" | undefined");
                    }
                    rendered.push_str("; ");
                }
                rendered.push('}');
                Some(rendered)
            }
            _ => None,
        }
    }

    fn object_property_annotation<'b>(
        &self,
        annotation: &'b TSType<'a>,
        name: &str,
    ) -> Option<&'b TSType<'a>>
    where
        'a: 'b,
    {
        self.object_annotation_members(annotation)?
            .iter()
            .find_map(|member| {
                let TSSignature::TSPropertySignature(property) = member else {
                    return None;
                };
                property
                    .key
                    .static_name()
                    .is_some_and(|property_name| property_name == name)
                    .then(|| property.type_annotation.as_ref())
                    .flatten()
                    .map(|annotation| &annotation.type_annotation)
            })
    }

    fn object_annotation_members<'b>(
        &self,
        annotation: &'b TSType<'a>,
    ) -> Option<&'b [TSSignature<'a>]>
    where
        'a: 'b,
    {
        let mut annotation = annotation;
        let mut remaining_named_types = self.aliases.len() + self.interfaces.len() + 1;
        loop {
            match annotation {
                TSType::TSTypeLiteral(literal) => return Some(&literal.members),
                TSType::TSTypeReference(reference) if reference.type_arguments.is_none() => {
                    let TSTypeName::IdentifierReference(identifier) = &reference.type_name else {
                        return None;
                    };
                    if remaining_named_types == 0 {
                        return None;
                    }
                    remaining_named_types -= 1;
                    let name = identifier.name.as_str();
                    if let Some(interface) = self.interfaces.get(name) {
                        return Some(&interface.body.body);
                    }
                    annotation = self.aliases.get(name).copied()?;
                }
                TSType::TSParenthesizedType(parenthesized) => {
                    annotation = &parenthesized.type_annotation;
                }
                _ => return None,
            }
        }
    }

    fn array_element_annotation<'b>(&self, annotation: &'b TSType<'a>) -> Option<&'b TSType<'a>>
    where
        'a: 'b,
    {
        let annotation = self.resolve_annotation(annotation)?;
        match annotation {
            TSType::TSArrayType(array) => Some(&array.element_type),
            _ => Self::generic_array_element(annotation),
        }
    }

    fn generic_array_element<'b>(annotation: &'b TSType<'a>) -> Option<&'b TSType<'a>> {
        let TSType::TSTypeReference(reference) = annotation else {
            return None;
        };
        let TSTypeName::IdentifierReference(identifier) = &reference.type_name else {
            return None;
        };
        if identifier.name != "Array" {
            return None;
        }
        let arguments = reference.type_arguments.as_ref()?;
        let [element] = arguments.params.as_slice() else {
            return None;
        };
        Some(element)
    }

    fn resolve_annotation<'b>(&self, annotation: &'b TSType<'a>) -> Option<&'b TSType<'a>>
    where
        'a: 'b,
    {
        let mut annotation = annotation;
        let mut remaining_aliases = self.aliases.len();
        loop {
            match annotation {
                TSType::TSTypeReference(reference) if reference.type_arguments.is_none() => {
                    let TSTypeName::IdentifierReference(identifier) = &reference.type_name else {
                        return Some(annotation);
                    };
                    let name = identifier.name.as_str();
                    if remaining_aliases == 0 {
                        return None;
                    }
                    remaining_aliases -= 1;
                    annotation = self.aliases.get(name).copied()?;
                }
                TSType::TSParenthesizedType(parenthesized) => {
                    annotation = &parenthesized.type_annotation;
                }
                _ => return Some(annotation),
            }
        }
    }

    fn assignment_target_text(&self, annotation: &TSType<'a>, target: TypeId) -> String {
        match self.types.kind(target) {
            Some(TypeKind::Union(_) | TypeKind::Object(_) | TypeKind::Array(_)) => {
                self.annotation_text(annotation).to_owned()
            }
            _ => self.types.display(target).to_string(),
        }
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
