use super::*;

/// Parses with this extension's own grammar — in real use the editor does
/// the parsing, so what these tests need is any tree of the right grammar.
fn parser(language: tree_sitter::Language) -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).expect("a bundled grammar must load");
    parser
}

fn parsed(source: &str) -> Tree {
    let mut parser = parser(tree_sitter_kotlin_ng::LANGUAGE.into());
    parser.parse(source, None).expect("parse")
}

#[test]
fn resolves_a_parameter_type() {
    let source = "class Foo {\n    fun run(param: Bar) {\n        val x = 0\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("val x").unwrap();
    assert_eq!(
        type_of_identifier_kotlin(&tree, source, cursor, "param"),
        Some("Bar".to_string())
    );
}

#[test]
fn resolves_an_explicit_type_local_without_touching_the_initializer() {
    // The initializer here (`someFunction()`) doesn't fit the
    // constructor-call heuristic at all — proving the explicit-type
    // branch never falls through to it.
    let source = "class Foo {\n    fun run() {\n        val local: Bar = someFunction()\n        val x = 0\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("val x").unwrap();
    assert_eq!(
        type_of_identifier_kotlin(&tree, source, cursor, "local"),
        Some("Bar".to_string())
    );
}

#[test]
fn resolves_an_inferred_type_local_via_the_constructor_call_heuristic() {
    let source = "class Foo {\n    fun run() {\n        val local = Bar()\n        val x = 0\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("val x").unwrap();
    assert_eq!(
        type_of_identifier_kotlin(&tree, source, cursor, "local"),
        Some("Bar".to_string())
    );
}

#[test]
fn an_inferred_type_local_via_a_non_constructor_call_resolves_to_none() {
    let source = "class Foo {\n    fun run() {\n        val local = someFunction()\n        val x = 0\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("val x").unwrap();
    assert_eq!(type_of_identifier_kotlin(&tree, source, cursor, "local"), None);
}

#[test]
fn resolves_a_field_type_when_nothing_shadows_it() {
    let source = "class Foo {\n    val field: Bar = Bar()\n    fun run() {\n        val x = 0\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("val x").unwrap();
    assert_eq!(
        type_of_identifier_kotlin(&tree, source, cursor, "field"),
        Some("Bar".to_string())
    );
}

#[test]
fn a_local_shadowing_a_field_resolves_to_the_local() {
    let source = "class Foo {\n    val shared: Bar = Bar()\n    fun run() {\n        val shared: Baz = Baz()\n        val x = 0\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("val x").unwrap();
    assert_eq!(
        type_of_identifier_kotlin(&tree, source, cursor, "shared"),
        Some("Baz".to_string())
    );
}

#[test]
fn resolves_a_primary_constructor_property_parameter() {
    let source = "class Foo(val x: Int) {\n    fun run() {\n        val y = 0\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("val y").unwrap();
    assert_eq!(
        type_of_identifier_kotlin(&tree, source, cursor, "x"),
        Some("Int".to_string())
    );
}

#[test]
fn an_undeclared_name_returns_none() {
    let source = "class Foo {\n    fun run() {\n        val x = 0\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("val x").unwrap();
    assert_eq!(type_of_identifier_kotlin(&tree, source, cursor, "neverDeclared"), None);
}
