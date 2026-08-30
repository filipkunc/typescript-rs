use std::{env, fs, path::Path};

use typescript_rs::check_source;

const REGISTERED_CASES: &[&str] = &[
    "array_empty.ts",
    "array_generic.ts",
    "array_generic_errors.ts",
    "array_incorrect_element_types.ts",
    "array_nested.ts",
    "array_object_element_properties.ts",
    "array_object_literals.ts",
    "array_primitive.ts",
    "callable_argument_errors.ts",
    "callable_arity_errors.ts",
    "callable_declarations_valid.ts",
    "callable_return_errors.ts",
    "callable_symbol_resolution.ts",
    "class_side_errors.ts",
    "class_sides_valid.ts",
    "contextual_union_shapes.ts",
    "diagnostic_assignment_anchors.ts",
    "incomplete_expression.ts",
    "incomplete_let_declaration.ts",
    "interface_callable_interactions.ts",
    "interface_member_access.ts",
    "interface_properties_valid.ts",
    "interface_structural_errors.ts",
    "json_shaped_values_complete.ts",
    "literal_union_aliases.ts",
    "negative_numeric_literals.ts",
    "nested_structural_diagnostics.ts",
    "object_type_excess_properties.ts",
    "object_type_literals_valid.ts",
    "object_type_missing_properties.ts",
    "object_type_missing_property.ts",
    "object_type_optional_properties.ts",
    "object_type_property_types.ts",
    "primitive_literals.ts",
    "recovered_malformed_expressions.ts",
    "recovered_missing_array_assignment_rhs.ts",
    "recovered_missing_array_spread_argument.ts",
    "recovered_missing_assignment_rhs.ts",
    "recovered_missing_call_argument.ts",
    "recovered_missing_call_assignment_rhs.ts",
    "recovered_missing_call_closer.ts",
    "recovered_missing_call_spread_argument.ts",
    "recovered_missing_class_closer.ts",
    "recovered_missing_class_member_separator.ts",
    "recovered_missing_declaration_name.ts",
    "recovered_missing_function_body_closer.ts",
    "recovered_missing_initializer.ts",
    "recovered_missing_interface_closer.ts",
    "recovered_missing_interface_member_separator.ts",
    "recovered_missing_interface_member_type.ts",
    "recovered_missing_list_closers.ts",
    "recovered_missing_list_commas.ts",
    "recovered_missing_member_name.ts",
    "recovered_missing_object_property_value.ts",
    "recovered_missing_parameter.ts",
    "recovered_missing_parameter_closer.ts",
    "recovered_missing_parameter_delimiters.ts",
    "recovered_missing_return_expression_operand.ts",
    "recovered_missing_return_type.ts",
    "recovered_missing_type_annotation.ts",
    "recovered_missing_type_delimiters.ts",
    "simple_assignment_errors.ts",
    "unexpected_statement_token.ts",
    "valid_primitive_literals.ts",
];

// Keep these as ordinary, explicitly named Rust tests. rust-analyzer can then
// expose each TypeScript fixture as its own item in VS Code's Test Explorer.
#[test]
fn primitive_literals() {
    run_case("primitive_literals.ts");
}

#[test]
fn recovered_missing_initializer() {
    run_case("recovered_missing_initializer.ts");
}

#[test]
fn recovered_malformed_expressions() {
    run_case("recovered_malformed_expressions.ts");
}

#[test]
fn recovered_missing_list_commas() {
    run_case("recovered_missing_list_commas.ts");
}

#[test]
fn recovered_missing_list_closers() {
    run_case("recovered_missing_list_closers.ts");
}

#[test]
fn recovered_missing_member_name() {
    run_case("recovered_missing_member_name.ts");
}

#[test]
fn recovered_missing_array_assignment_rhs() {
    run_case("recovered_missing_array_assignment_rhs.ts");
}

#[test]
fn recovered_missing_array_spread_argument() {
    run_case("recovered_missing_array_spread_argument.ts");
}

#[test]
fn recovered_missing_call_argument() {
    run_case("recovered_missing_call_argument.ts");
}

#[test]
fn recovered_missing_call_assignment_rhs() {
    run_case("recovered_missing_call_assignment_rhs.ts");
}

#[test]
fn recovered_missing_call_closer() {
    run_case("recovered_missing_call_closer.ts");
}

#[test]
fn recovered_missing_call_spread_argument() {
    run_case("recovered_missing_call_spread_argument.ts");
}

#[test]
fn recovered_missing_class_member_separator() {
    run_case("recovered_missing_class_member_separator.ts");
}

#[test]
fn recovered_missing_class_closer() {
    run_case("recovered_missing_class_closer.ts");
}

#[test]
fn recovered_missing_declaration_name() {
    run_case("recovered_missing_declaration_name.ts");
}

#[test]
fn recovered_missing_function_body_closer() {
    run_case("recovered_missing_function_body_closer.ts");
}

#[test]
fn recovered_missing_interface_closer() {
    run_case("recovered_missing_interface_closer.ts");
}

#[test]
fn recovered_missing_interface_member_type() {
    run_case("recovered_missing_interface_member_type.ts");
}

#[test]
fn recovered_missing_interface_member_separator() {
    run_case("recovered_missing_interface_member_separator.ts");
}

#[test]
fn recovered_missing_assignment_rhs() {
    run_case("recovered_missing_assignment_rhs.ts");
}

#[test]
fn recovered_missing_object_property_value() {
    run_case("recovered_missing_object_property_value.ts");
}

#[test]
fn recovered_missing_parameter_delimiters() {
    run_case("recovered_missing_parameter_delimiters.ts");
}

#[test]
fn recovered_missing_parameter() {
    run_case("recovered_missing_parameter.ts");
}

#[test]
fn recovered_missing_parameter_closer() {
    run_case("recovered_missing_parameter_closer.ts");
}

#[test]
fn recovered_missing_return_expression_operand() {
    run_case("recovered_missing_return_expression_operand.ts");
}

#[test]
fn recovered_missing_return_type() {
    run_case("recovered_missing_return_type.ts");
}

#[test]
fn recovered_missing_type_annotation() {
    run_case("recovered_missing_type_annotation.ts");
}

#[test]
fn recovered_missing_type_delimiters() {
    run_case("recovered_missing_type_delimiters.ts");
}

#[test]
fn literal_union_aliases() {
    run_case("literal_union_aliases.ts");
}

#[test]
fn valid_primitive_literals() {
    run_case("valid_primitive_literals.ts");
}

#[test]
fn object_type_literals_valid() {
    run_case("object_type_literals_valid.ts");
}

#[test]
fn object_type_missing_property() {
    run_case("object_type_missing_property.ts");
}

#[test]
fn object_type_missing_properties() {
    run_case("object_type_missing_properties.ts");
}

#[test]
fn object_type_optional_properties() {
    run_case("object_type_optional_properties.ts");
}

#[test]
fn object_type_property_types() {
    run_case("object_type_property_types.ts");
}

#[test]
fn array_primitive() {
    run_case("array_primitive.ts");
}

#[test]
fn array_object_literals() {
    run_case("array_object_literals.ts");
}

#[test]
fn array_nested() {
    run_case("array_nested.ts");
}

#[test]
fn array_incorrect_element_types() {
    run_case("array_incorrect_element_types.ts");
}

#[test]
fn array_object_element_properties() {
    run_case("array_object_element_properties.ts");
}

#[test]
fn array_empty() {
    run_case("array_empty.ts");
}

#[test]
fn array_generic() {
    run_case("array_generic.ts");
}

#[test]
fn array_generic_errors() {
    run_case("array_generic_errors.ts");
}

#[test]
fn callable_argument_errors() {
    run_case("callable_argument_errors.ts");
}

#[test]
fn callable_arity_errors() {
    run_case("callable_arity_errors.ts");
}

#[test]
fn callable_declarations_valid() {
    run_case("callable_declarations_valid.ts");
}

#[test]
fn callable_return_errors() {
    run_case("callable_return_errors.ts");
}

#[test]
fn callable_symbol_resolution() {
    run_case("callable_symbol_resolution.ts");
}

#[test]
fn class_side_errors() {
    run_case("class_side_errors.ts");
}

#[test]
fn class_sides_valid() {
    run_case("class_sides_valid.ts");
}

#[test]
fn contextual_union_shapes() {
    run_case("contextual_union_shapes.ts");
}

#[test]
fn diagnostic_assignment_anchors() {
    run_case("diagnostic_assignment_anchors.ts");
}

#[test]
fn interface_callable_interactions() {
    run_case("interface_callable_interactions.ts");
}

#[test]
fn interface_member_access() {
    run_case("interface_member_access.ts");
}

#[test]
fn interface_properties_valid() {
    run_case("interface_properties_valid.ts");
}

#[test]
fn interface_structural_errors() {
    run_case("interface_structural_errors.ts");
}

#[test]
fn incomplete_expression() {
    run_case("incomplete_expression.ts");
}

#[test]
fn incomplete_let_declaration() {
    run_case("incomplete_let_declaration.ts");
}

#[test]
fn json_shaped_values_complete() {
    run_case("json_shaped_values_complete.ts");
}

#[test]
fn negative_numeric_literals() {
    run_case("negative_numeric_literals.ts");
}

#[test]
fn nested_structural_diagnostics() {
    run_case("nested_structural_diagnostics.ts");
}

#[test]
fn object_type_excess_properties() {
    run_case("object_type_excess_properties.ts");
}

#[test]
fn simple_assignment_errors() {
    run_case("simple_assignment_errors.ts");
}

#[test]
fn unexpected_statement_token() {
    run_case("unexpected_statement_token.ts");
}

#[test]
fn every_fixture_has_a_registered_test() {
    let root = cases_root();
    let mut discovered: Vec<_> = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ts"))
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    discovered.sort();

    assert_eq!(
        discovered, REGISTERED_CASES,
        "add a named #[test] and REGISTERED_CASES entry for every fixture so it appears in Test Explorer"
    );
}

fn run_case(relative_path: &str) {
    let case = cases_root().join(relative_path);
    let source = fs::read_to_string(&case)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", case.display()));
    let actual = check_source(&case.to_string_lossy(), &source).render_concise(&source);
    let baseline = case.with_extension("errors");

    if env::var_os("BLESS").is_some() {
        fs::write(&baseline, &actual)
            .unwrap_or_else(|error| panic!("failed to update {}: {error}", baseline.display()));
        return;
    }

    let expected = fs::read_to_string(&baseline).unwrap_or_default();
    if actual != expected {
        let actual_path = case.with_extension("actual");
        fs::write(&actual_path, &actual)
            .unwrap_or_else(|error| panic!("failed to write {}: {error}", actual_path.display()));
        panic!(
            "{}\nexpected:\n{}\nactual:\n{}\nfull output: {}",
            case.display(),
            expected,
            actual,
            actual_path.display()
        );
    }
}

fn cases_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases/compiler")
}
