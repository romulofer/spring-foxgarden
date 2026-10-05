use super::*;

/// Parses with this extension's own grammar — in real use the editor does
/// the parsing, so what these tests need is any tree of the right grammar.
fn parser(language: tree_sitter::Language) -> tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).expect("a bundled grammar must load");
    parser
}

fn parsed(source: &str) -> Tree {
    let mut parser = parser(tree_sitter_java::LANGUAGE.into());
    parser.parse(source, None).expect("parse")
}

#[test]
fn resolves_a_parameter_type() {
    let source = "class Foo {\n    void run(Bar param) {\n        int x = 0;\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("int x").unwrap();
    assert_eq!(
        type_of_identifier_java(&tree, source, cursor, "param"),
        Some("Bar".to_string())
    );
}

#[test]
fn resolves_a_local_variables_type() {
    let source = "class Foo {\n    void run() {\n        Bar local = new Bar();\n        int x = 0;\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("int x").unwrap();
    assert_eq!(
        type_of_identifier_java(&tree, source, cursor, "local"),
        Some("Bar".to_string())
    );
}

#[test]
fn resolves_a_field_type_when_nothing_shadows_it() {
    let source = "class Foo {\n    private Bar field;\n    void run() {\n        int x = 0;\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("int x").unwrap();
    assert_eq!(
        type_of_identifier_java(&tree, source, cursor, "field"),
        Some("Bar".to_string())
    );
}

#[test]
fn a_local_shadowing_a_field_resolves_to_the_local() {
    let source = "class Foo {\n    private Bar shared;\n    void run() {\n        Baz shared = new Baz();\n        int x = 0;\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("int x").unwrap();
    assert_eq!(
        type_of_identifier_java(&tree, source, cursor, "shared"),
        Some("Baz".to_string())
    );
}

#[test]
fn an_undeclared_name_returns_none() {
    let source = "class Foo {\n    void run() {\n        int x = 0;\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("int x").unwrap();
    assert_eq!(type_of_identifier_java(&tree, source, cursor, "neverDeclared"), None);
}

#[test]
fn a_jdk_typed_local_resolves_syntactically_like_any_other_type() {
    // No classpath awareness at this layer — filtering JDK types out
    // is `SPEC.md` §4's job (the resolved name fails to find a
    // matching project source file), not this function's.
    let source = "class Foo {\n    void run() {\n        String s = \"hi\";\n        int x = 0;\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("int x").unwrap();
    assert_eq!(
        type_of_identifier_java(&tree, source, cursor, "s"),
        Some("String".to_string())
    );
}

#[test]
fn strips_generics_and_package_qualification_same_as_superclass_name() {
    let source = "class Foo {\n    void run(java.util.List<String> items) {\n        int x = 0;\n    }\n}\n";
    let tree = parsed(source);
    let cursor = source.find("int x").unwrap();
    assert_eq!(
        type_of_identifier_java(&tree, source, cursor, "items"),
        Some("List".to_string())
    );
}

#[test]
fn returns_none_outside_any_class() {
    let source = "// just a comment\n";
    let tree = parsed(source);
    assert_eq!(type_of_identifier_java(&tree, source, 0, "x"), None);
}

/// A local `val` shadows a constructor property of the same name.
#[test]
fn kotlin_local_val_shadows_a_constructor_property() {
    let source = "class A(val repo: Repo) {\n    fun f() {\n        val repo = Other()\n        repo.\n    }\n}\n";
    let mut parser = parser(tree_sitter_kotlin_ng::LANGUAGE.into());
    let tree = parser.parse(source, None).expect("parse");
    let cursor = source.find("repo.").unwrap();
    assert_eq!(type_of_identifier_kotlin(&tree, source, cursor, "repo").as_deref(), Some("Other"));
}
