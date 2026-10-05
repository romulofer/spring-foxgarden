use super::*;

fn parse(language: tree_sitter::Language, source: &str) -> Tree {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).expect("a bundled grammar must load");
    parser.parse(source, None).expect("parse")
}

fn java(source: &str) -> Tree {
    parse(tree_sitter_java::LANGUAGE.into(), source)
}

fn kotlin(source: &str) -> Tree {
    parse(tree_sitter_kotlin_ng::LANGUAGE.into(), source)
}

fn names(members: &[TypeMember]) -> Vec<&str> {
    members.iter().map(|m| m.name.as_str()).collect()
}

const JAVA_CLASS: &str = "class Foo extends Base {\n    private int x;\n    private static final int MAX = 1;\n    private void helper() {\n    }\n    public final void locked() {\n    }\n    public String run(int times) {\n        return null;\n    }\n}\n";

#[test]
fn java_inside_view_sees_every_member_fields_first() {
    let tree = java(JAVA_CLASS);
    let members = type_members(JAVA, &tree, JAVA_CLASS, "Foo", MemberView::Inside);
    assert_eq!(names(&members), ["x", "MAX", "helper", "locked", "run"]);
}

#[test]
fn java_outside_view_drops_statics_privates_and_finals() {
    let tree = java(JAVA_CLASS);
    let members = type_members(JAVA, &tree, JAVA_CLASS, "Foo", MemberView::Outside);
    assert_eq!(names(&members), ["x", "run"]);
}

#[test]
fn java_overridable_view_is_methods_only() {
    let tree = java(JAVA_CLASS);
    let members = type_members(JAVA, &tree, JAVA_CLASS, "Foo", MemberView::Overridable);
    assert_eq!(names(&members), ["run"]);
    assert_eq!(members[0].kind, MemberKind::Method);
    assert_eq!(members[0].type_text, "String");
    assert_eq!(members[0].params, [("int".to_string(), "times".to_string())]);
}

#[test]
fn a_final_field_is_read_only() {
    let source = "class Foo {\n    private final String name;\n    private int age;\n}\n";
    let tree = java(source);
    let members = type_members(JAVA, &tree, source, "Foo", MemberView::Inside);
    let read_only: Vec<(&str, bool)> = members.iter().map(|m| (m.name.as_str(), m.read_only)).collect();
    assert_eq!(read_only, [("name", true), ("age", false)]);
}

#[test]
fn java_receivers_resolve_this_super_and_variables() {
    let source = "class Foo extends Base {\n    private Bar bar;\n    void go(Baz baz) {\n        int here = 0;\n    }\n}\n";
    let tree = java(source);
    let byte = source.find("int here").unwrap();

    let this = receiver_type(JAVA, &tree, source, byte, "this").unwrap();
    assert_eq!((this.type_name.as_str(), this.view, this.in_this_file), ("Foo", MemberView::Inside, true));

    let sup = receiver_type(JAVA, &tree, source, byte, "super").unwrap();
    assert_eq!((sup.type_name.as_str(), sup.view, sup.in_this_file), ("Base", MemberView::Inside, false));

    let field = receiver_type(JAVA, &tree, source, byte, "bar").unwrap();
    assert_eq!((field.type_name.as_str(), field.view), ("Bar", MemberView::Outside));

    let param = receiver_type(JAVA, &tree, source, byte, "baz").unwrap();
    assert_eq!(param.type_name, "Baz");

    assert_eq!(receiver_type(JAVA, &tree, source, byte, "nowhere"), None);
}

#[test]
fn super_without_a_supertype_resolves_to_nothing() {
    let source = "class Foo {\n    void go() {\n    }\n}\n";
    let tree = java(source);
    let byte = source.find("go").unwrap();
    assert_eq!(receiver_type(JAVA, &tree, source, byte, "super"), None);
}

#[test]
fn kotlin_receivers_resolve_like_javas() {
    let source = "class Foo : Base() {\n    fun run(param: Bar) {\n        val x = 0\n    }\n}\n";
    let tree = kotlin(source);
    let byte = source.find("val x").unwrap();

    assert_eq!(receiver_type(KOTLIN, &tree, source, byte, "this").unwrap().type_name, "Foo");
    assert_eq!(receiver_type(KOTLIN, &tree, source, byte, "super").unwrap().type_name, "Base");
    let param = receiver_type(KOTLIN, &tree, source, byte, "param").unwrap();
    assert_eq!((param.type_name.as_str(), param.view), ("Bar", MemberView::Outside));
}

#[test]
fn kotlin_outside_view_hides_private_functions_but_not_properties() {
    let source = "class Foo(val id: Int) {\n    var name: String = \"\"\n    fun run() {}\n    private fun hidden() {}\n}\n";
    let tree = kotlin(source);
    let outside = type_members(KOTLIN, &tree, source, "Foo", MemberView::Outside);
    assert_eq!(names(&outside), ["id", "name", "run"]);
    let inside = type_members(KOTLIN, &tree, source, "Foo", MemberView::Inside);
    assert_eq!(names(&inside), ["id", "name", "run", "hidden"]);
    assert!(type_members(KOTLIN, &tree, source, "Foo", MemberView::Overridable).is_empty());
}

#[test]
fn enclosing_type_names_where_generated_code_goes() {
    let source = "class Foo {\n    void run() {\n    }\n}\n";
    let tree = java(source);
    let declaration = enclosing_type(JAVA, &tree, source, source.find("run").unwrap()).unwrap();
    assert_eq!(declaration.name, "Foo");
    let byte = declaration.insertion_byte.unwrap();
    assert_eq!(&source[byte..byte + 1], "}");
}

#[test]
fn a_kotlin_class_without_a_body_has_nowhere_to_insert() {
    let source = "class Id(val value: Int)\n";
    let tree = kotlin(source);
    let declaration = enclosing_type(KOTLIN, &tree, source, source.find("value").unwrap()).unwrap();
    assert_eq!(declaration.name, "Id");
    assert_eq!(declaration.insertion_byte, None);

    let with_body = "class Foo {\n    fun run() {}\n}\n";
    let tree = kotlin(with_body);
    let declaration = enclosing_type(KOTLIN, &tree, with_body, with_body.find("run").unwrap()).unwrap();
    let byte = declaration.insertion_byte.unwrap();
    assert_eq!(&with_body[byte..byte + 1], "}");
}

#[test]
fn types_with_fields_are_javas_alone() {
    let source = "class Foo {\n    private int x;\n}\n";
    let tree = java(source);
    let types = types_with_fields(JAVA, &tree, source);
    assert_eq!(types.len(), 1);
    assert_eq!(types[0].declaration.name, "Foo");
    assert_eq!(names(&types[0].fields), ["x"]);

    let kotlin_source = "class Foo(val x: Int)\n";
    let tree = kotlin(kotlin_source);
    assert!(types_with_fields(KOTLIN, &tree, kotlin_source).is_empty());
}

#[test]
fn another_extensions_language_gets_no_answers() {
    let tree = java(JAVA_CLASS);
    assert_eq!(enclosing_type("python", &tree, JAVA_CLASS, 20), None);
    assert_eq!(supertype("python", &tree, JAVA_CLASS, "Foo"), None);
    assert!(type_members("python", &tree, JAVA_CLASS, "Foo", MemberView::Inside).is_empty());
    assert_eq!(receiver_type("python", &tree, JAVA_CLASS, 20, "this"), None);
}
