use super::*;

fn kotlin(source: &str) -> Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_kotlin_ng::LANGUAGE.into())
        .expect("a bundled grammar must load");
    parser.parse(source, None).expect("parse")
}

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

/// Like `stubbed`, for Kotlin.
fn kdoc(marked: &str) -> Option<String> {
    let byte = marked.find('^').expect("a ^ marker");
    let source = marked.replacen('^', "", 1);
    let tree = kotlin(&source);
    let stub = doc_stub(&tree, &source, KOTLIN, byte)?;
    let mut result = source;
    result.insert_str(stub.byte, &stub.text);
    Some(result)
}

#[test]
fn a_kotlin_function_gets_params_and_a_return() {
    let result = kdoc("class A {\n    fun add(a: Int, vararg rest: String): Int {\n        ^return 1\n    }\n}\n").unwrap();
    assert_eq!(
        result,
        "class A {\n    /**\n     * \n     *\n     * @param a\n     * @param rest\n     * @return\n     */\n    fun add(a: Int, vararg rest: String): Int {\n        return 1\n    }\n}\n"
    );
}

#[test]
fn a_unit_function_has_no_return() {
    let explicit = kdoc("fun run(a: Int): Unit {^}\n").unwrap();
    let implicit = kdoc("fun run(a: Int) {^}\n").unwrap();
    assert!(!explicit.contains("@return"), "{explicit}");
    assert!(!implicit.contains("@return"), "{implicit}");
}

#[test]
fn an_expression_body_without_a_declared_type_is_assumed_to_return_something() {
    let result = kdoc("fun twice(a: Int) = a * 2^\n").unwrap();
    assert!(result.contains(" * @return\n"), "{result}");
}

#[test]
fn a_class_documents_its_type_parameters_and_primary_constructor() {
    let result = kdoc("class Box<T>(val a: Int, b: String) {\n    ^\n}\n").unwrap();
    assert_eq!(
        result,
        "/**\n * \n *\n * @param T\n * @property a\n * @param b\n */\nclass Box<T>(val a: Int, b: String) {\n    \n}\n"
    );
}

#[test]
fn a_kotlin_function_documents_its_type_parameters() {
    let result = kdoc("fun <R> run(x: Int): R {^}\n").unwrap();
    assert!(result.contains(" * @param R\n * @param x\n * @return\n"), "{result}");
}

#[test]
fn a_secondary_constructor_has_no_return() {
    let result = kdoc("class A(x: Int) {\n    constructor(q: Int) : this(q) {^}\n}\n").unwrap();
    assert!(result.contains("     * @param q\n"), "{result}");
    assert!(!result.contains("@return"), "{result}");
}

#[test]
fn an_object_is_documentable() {
    let result = kdoc("object Registry {\n    ^\n}\n").unwrap();
    assert!(result.starts_with("/**\n * \n */\nobject Registry"), "{result}");
}

#[test]
fn an_already_documented_kotlin_declaration_is_left_alone() {
    assert_eq!(kdoc("/** Runs. */\nfun run() {^}\n"), None);
}

#[test]
fn a_kotlin_line_comment_does_not_count_as_documentation() {
    assert!(kdoc("// runs\nfun run() {^}\n").is_some());
}

#[test]
fn kotlin_outside_any_declaration_yields_nothing() {
    assert_eq!(kdoc("^package demo\n\nclass A\n"), None);
}
