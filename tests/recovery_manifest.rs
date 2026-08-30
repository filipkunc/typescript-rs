use std::{collections::BTreeSet, fs, path::Path};

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{BindingPattern, Program, Statement},
};
use oxc_ast_visit::Visit;
use oxc_parser::{ParseMode, ParseOptions, Parser};
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use serde::Deserialize;

const PINNED_REVISION: &str = include_str!("tsgo-reference.txt");
const SUPPORTED_CASES: &[&str] = &[
    "malformed_expression",
    "missing_array_operand",
    "missing_assignment_rhs",
    "missing_call_argument",
    "missing_call_closer",
    "missing_class_closer",
    "missing_class_member",
    "missing_declaration_name",
    "missing_function_body_closer",
    "missing_initializer",
    "missing_initializer_before_newline_declaration",
    "missing_initializer_before_same_line_declaration",
    "missing_interface_closer",
    "missing_interface_member_separator",
    "missing_interface_member_type",
    "missing_list_delimiters",
    "missing_member_name",
    "missing_object_value",
    "missing_optional_member_name",
    "missing_parameter",
    "missing_parameter_closer",
    "missing_parameter_closer_eof",
    "missing_parameter_delimiter",
    "missing_parameter_type",
    "missing_return_expression_operand",
    "missing_return_type",
    "missing_type",
];

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReferenceManifest {
    schema_version: u32,
    implementation: String,
    revision: String,
    cases: Vec<ReferenceCase>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReferenceCase {
    id: String,
    source: String,
    top_level: Vec<ReferenceTopLevel>,
    declaration_names: Vec<String>,
    binding_names: Vec<String>,
    diagnostics: Vec<ReferenceDiagnostic>,
    recovery_nodes: Vec<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ReferenceTopLevel {
    kind: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct ReferenceDiagnostic {
    code: u32,
    start: u32,
    message: String,
}

#[derive(Debug, PartialEq, Eq)]
struct OxcDiagnostic {
    code: u32,
    start: u32,
    message: String,
}

#[derive(Default)]
struct RecoveryCounter {
    count: usize,
}

impl<'a> Visit<'a> for RecoveryCounter {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        if matches!(
            kind,
            AstKind::MissingExpression(_)
                | AstKind::MalformedExpression(_)
                | AstKind::MissingMemberExpression(_)
                | AstKind::MissingType(_)
        ) {
            self.count += 1;
        }
    }
}

#[test]
fn pinned_typescript_go_recovery_manifest_is_current_and_complete() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = read_manifest(root);
    assert_eq!(manifest.schema_version, 1);
    assert_eq!(manifest.implementation, "typescript-go");
    let pinned_revision = PINNED_REVISION
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .expect("pinned TypeScript-Go revision");
    assert_eq!(manifest.revision, pinned_revision);

    let cases_root = root.join("tests/recovery-manifest/cases");
    let expected_ids: BTreeSet<_> = fs::read_dir(&cases_root)
        .expect("read recovery-manifest cases")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ts"))
        .map(|path| {
            path.file_stem()
                .expect("case file stem")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    let actual_ids: BTreeSet<_> = manifest.cases.iter().map(|case| case.id.clone()).collect();
    assert_eq!(actual_ids, expected_ids);

    for case in &manifest.cases {
        let source = fs::read_to_string(cases_root.join(format!("{}.ts", case.id)))
            .expect("read recovery-manifest case");
        assert_eq!(
            case.source, source,
            "regenerate manifest after editing {}",
            case.id
        );
        if !case.diagnostics.is_empty() {
            assert!(
                !case.recovery_nodes.is_empty(),
                "{} must expose its parse-error propagation or missing nodes",
                case.id
            );
        }
    }
}

#[test]
fn supported_oxc_recovery_matches_the_pinned_manifest_dimensions() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = read_manifest(root);

    for id in SUPPORTED_CASES {
        let reference = manifest
            .cases
            .iter()
            .find(|case| case.id == *id)
            .unwrap_or_else(|| panic!("missing reference manifest case {id}"));
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, &reference.source, SourceType::ts())
            .with_options(ParseOptions {
                mode: ParseMode::Editor,
                ..ParseOptions::default()
            })
            .parse();
        assert!(!parsed.panicked, "Oxc editor parse aborted for {id}");

        let diagnostics: Vec<_> = parsed
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let label = diagnostic.labels.first().expect("parser diagnostic label");
                OxcDiagnostic {
                    code: diagnostic
                        .code
                        .number
                        .as_deref()
                        .expect("TypeScript diagnostic number")
                        .parse()
                        .expect("numeric TypeScript diagnostic code"),
                    start: label.offset(),
                    message: diagnostic.message.to_string(),
                }
            })
            .collect();
        let reference_diagnostics: Vec<_> = reference
            .diagnostics
            .iter()
            .map(|diagnostic| OxcDiagnostic {
                code: diagnostic.code,
                start: diagnostic.start,
                message: diagnostic.message.clone(),
            })
            .collect();
        assert_eq!(
            diagnostics, reference_diagnostics,
            "parser diagnostics differ for {id}"
        );

        let top_level: Vec<_> = parsed
            .program
            .body
            .iter()
            .map(normalize_statement_kind)
            .collect();
        let reference_top_level: Vec<_> = reference
            .top_level
            .iter()
            .filter(|node| node.kind != "EmptyStatement")
            .map(|node| node.kind.as_str())
            .collect();
        assert_eq!(
            top_level, reference_top_level,
            "surviving statement kinds differ for {id}"
        );

        let declarations = declaration_names(&parsed.program);
        assert_eq!(
            declarations, reference.declaration_names,
            "declarations differ for {id}"
        );

        let built = SemanticBuilder::new_compiler().build(&parsed.program);
        assert!(
            built.diagnostics.is_empty(),
            "semantic build failed for {id}"
        );
        let root_scope = built.semantic.scoping().root_scope_id();
        let mut bindings: Vec<_> = built
            .semantic
            .scoping()
            .get_bindings(root_scope)
            .keys()
            .map(ToString::to_string)
            .collect();
        bindings.sort();
        assert_eq!(
            bindings, reference.binding_names,
            "semantic bindings differ for {id}"
        );

        let mut recovery = RecoveryCounter::default();
        recovery.visit_program(&parsed.program);
        recovery.count += parsed.recoveries.len();
        assert_eq!(
            recovery.count,
            reference.diagnostics.len(),
            "every supported diagnostic must have one explicit Oxc recovery site for {id}"
        );
    }
}

fn read_manifest(root: &Path) -> ReferenceManifest {
    let json = fs::read_to_string(root.join("tests/recovery-manifest/typescript-go.json"))
        .expect("read pinned TypeScript-Go recovery manifest");
    serde_json::from_str(&json).expect("parse pinned TypeScript-Go recovery manifest")
}

fn normalize_statement_kind(statement: &Statement<'_>) -> &'static str {
    match statement {
        Statement::VariableDeclaration(_) => "variable",
        Statement::ExpressionStatement(_) => "expression",
        Statement::FunctionDeclaration(_) => "function",
        Statement::TSInterfaceDeclaration(_) => "interface",
        Statement::ClassDeclaration(_) => "class",
        Statement::TSTypeAliasDeclaration(_) => "typeAlias",
        _ => "other",
    }
}

fn declaration_names(program: &Program<'_>) -> Vec<String> {
    let mut names = Vec::new();
    for statement in &program.body {
        match statement {
            Statement::VariableDeclaration(declaration) => {
                names.extend(declaration.declarations.iter().filter_map(|declarator| {
                    let BindingPattern::BindingIdentifier(identifier) = &declarator.id else {
                        return None;
                    };
                    Some(identifier.name.to_string())
                }));
            }
            Statement::FunctionDeclaration(function) => {
                names.extend(
                    function
                        .id
                        .iter()
                        .map(|identifier| identifier.name.to_string()),
                );
            }
            Statement::ClassDeclaration(class) => {
                names.extend(
                    class
                        .id
                        .iter()
                        .map(|identifier| identifier.name.to_string()),
                );
            }
            Statement::TSInterfaceDeclaration(interface) => {
                names.push(interface.id.name.to_string());
            }
            Statement::TSTypeAliasDeclaration(alias) => names.push(alias.id.name.to_string()),
            _ => {}
        }
    }
    names.sort();
    names
}
