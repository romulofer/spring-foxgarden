use super::*;

fn java(source: &str) -> Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .expect("a bundled grammar must load");
    parser.parse(source, None).expect("parse")
}

/// `source` with the stub for the declaration at the first `^` marker spliced
/// in, the marker removed.
fn stubbed(marked: &str) -> Option<String> {
    let byte = marked.find('^').expect("a ^ marker");
    let source = marked.replacen('^', "", 1);
    let tree = java(&source);
    let stub = doc_stub(&tree, &source, JAVA, byte)?;
    let mut result = source;
    result.insert_str(stub.byte, &stub.text);
    Some(result)
}

#[test]
fn a_method_gets_a_param_return_and_throws_line() {
    let result = stubbed(
        "class A {\n    public int div(int a, String... rest) throws IOException, X {\n        ^return 1;\n    }\n}\n",
    )
    .unwrap();
    assert_eq!(
        result,
        "class A {\n    /**\n     * \n     *\n     * @param a\n     * @param rest\n     * @return\n     * @throws IOException\n     * @throws X\n     */\n    public int div(int a, String... rest) throws IOException, X {\n        return 1;\n    }\n}\n"
    );
}

#[test]
fn a_void_method_has_no_return_line() {
    let result = stubbed("class A {\n    void run(int n) {^}\n}\n").unwrap();
    assert!(result.contains(" * @param n\n"), "{result}");
    assert!(!result.contains("@return"), "{result}");
}

#[test]
fn a_method_with_nothing_to_describe_gets_a_bare_comment() {
    let result = stubbed("class A {\n    void run() {^}\n}\n").unwrap();
    assert_eq!(result, "class A {\n    /**\n     * \n     */\n    void run() {}\n}\n");
}

#[test]
fn a_constructor_has_no_return_line() {
    let result = stubbed("class A {\n    A(int x) {^}\n}\n").unwrap();
    assert!(result.contains("@param x"), "{result}");
    assert!(!result.contains("@return"), "{result}");
}

#[test]
fn generic_type_parameters_are_documented_first() {
    let result = stubbed("class Box<T, U> {\n^}\n").unwrap();
    assert_eq!(
        result,
        "/**\n * \n *\n * @param <T>\n * @param <U>\n */\nclass Box<T, U> {\n}\n"
    );
}

#[test]
fn a_class_at_column_zero_is_documented_above_its_modifiers_and_annotations() {
    let result = stubbed("@Service\npublic class A {\n    ^\n}\n").unwrap();
    assert!(result.starts_with("/**\n * \n */\n@Service\npublic class A"), "{result}");
}

#[test]
fn the_innermost_declaration_wins() {
    let result = stubbed("class A {\n    class B {\n        ^\n    }\n}\n").unwrap();
    assert!(result.contains("    /**\n     * \n     */\n    class B"), "{result}");
    assert!(!result.starts_with("/**"), "{result}");
}

#[test]
fn a_record_documents_its_components() {
    let result = stubbed("record P(int x, int y) {^}\n").unwrap();
    assert!(result.contains(" * @param x\n * @param y\n"), "{result}");
}

#[test]
fn an_already_documented_declaration_is_left_alone() {
    assert_eq!(stubbed("class A {\n    /** Runs. */\n    void run() {^}\n}\n"), None);
}

#[test]
fn a_plain_comment_does_not_count_as_documentation() {
    assert!(stubbed("class A {\n    // runs\n    void run() {^}\n}\n").is_some());
}

#[test]
fn the_cursor_outside_any_declaration_yields_nothing() {
    assert_eq!(stubbed("^package demo;\n\nclass A {}\n"), None);
}

#[test]
fn another_language_yields_nothing() {
    let tree = java("class A {}");
    assert_eq!(doc_stub(&tree, "class A {}", "yaml", 0), None);
}
