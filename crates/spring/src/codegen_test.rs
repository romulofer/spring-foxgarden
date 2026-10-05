use fg_extension::MemberKind;

use super::*;

fn field(name: &str, java_type: &str, is_final: bool) -> TypeMember {
    TypeMember {
        name: name.to_string(),
        kind: MemberKind::Field,
        type_text: java_type.to_string(),
        params: Vec::new(),
        read_only: is_final,
    }
}

fn method(name: &str, return_type: &str, params: Vec<(&str, &str)>) -> TypeMember {
    TypeMember {
        name: name.to_string(),
        kind: MemberKind::Method,
        type_text: return_type.to_string(),
        params: params
            .into_iter()
            .map(|(t, n)| (t.to_string(), n.to_string()))
            .collect(),
        read_only: false,
    }
}

#[test]
fn generates_getter_and_setter_for_a_mutable_field() {
    let generated = accessors(&[field("name", "String", false)], "    ", true, true);
    assert_eq!(
        generated,
        "    public String getName() {\n\
             \x20\x20\x20\x20\x20\x20\x20\x20return this.name;\n\
             \x20\x20\x20\x20}\n\
             \n\
             \x20\x20\x20\x20public void setName(String name) {\n\
             \x20\x20\x20\x20\x20\x20\x20\x20this.name = name;\n\
             \x20\x20\x20\x20}\n"
    );
}

#[test]
fn both_generates_only_a_getter_for_a_final_field() {
    let generated = accessors(&[field("id", "int", true)], "  ", true, true);
    assert_eq!(generated, "  public int getId() {\n    return this.id;\n  }\n");
    assert!(!generated.contains("setId"));
}

#[test]
fn getters_alone_generate_only_getters() {
    let generated = accessors(&[field("name", "String", false)], "    ", true, false);
    assert!(generated.contains("getName"));
    assert!(!generated.contains("setName"));
}

#[test]
fn setters_alone_generate_only_setters() {
    let generated = accessors(&[field("name", "String", false)], "    ", false, true);
    assert!(!generated.contains("getName"));
    assert!(generated.contains("setName"));
}

#[test]
fn setters_alone_on_an_all_final_class_produce_nothing() {
    let generated = accessors(&[field("id", "int", true)], "    ", false, true);
    assert_eq!(generated, "");
}

#[test]
fn separates_multiple_fields_with_a_blank_line() {
    let generated = accessors(&[field("x", "int", true), field("y", "int", true)], "  ", true, true);
    assert_eq!(
        generated,
        "  public int getX() {\n    return this.x;\n  }\n\n  public int getY() {\n    return this.y;\n  }\n"
    );
}

#[test]
fn separates_multiple_fields_with_a_blank_line_for_getters_only() {
    let generated = accessors(&[field("x", "int", false), field("y", "int", false)], "  ", true, false);
    assert_eq!(
        generated,
        "  public int getX() {\n    return this.x;\n  }\n\n  public int getY() {\n    return this.y;\n  }\n"
    );
}

#[test]
fn capitalizes_only_the_first_character() {
    let generated = accessors(&[field("userId", "long", false)], "", true, true);
    assert!(generated.contains("getUserId"));
    assert!(generated.contains("setUserId"));
}

#[test]
fn empty_fields_produces_empty_output() {
    assert_eq!(accessors(&[], "    ", true, true), "");
}

#[test]
fn constructor_for_assigns_every_field_from_a_same_named_parameter() {
    let generated = constructor_for("Point", &[field("x", "int", false), field("y", "int", false)], "    ");
    assert_eq!(
        generated,
        "    public Point(int x, int y) {\n\
             \x20\x20\x20\x20\x20\x20\x20\x20this.x = x;\n\
             \x20\x20\x20\x20\x20\x20\x20\x20this.y = y;\n\
             \x20\x20\x20\x20}\n"
    );
}

#[test]
fn constructor_for_with_no_fields_is_still_valid_java() {
    let generated = constructor_for("Empty", &[], "    ");
    assert_eq!(generated, "    public Empty() {\n    }\n");
}

#[test]
fn to_string_for_concatenates_every_field() {
    let generated = to_string_for("Point", &[field("x", "int", false), field("y", "int", false)], "    ");
    assert!(generated.contains("@Override"));
    assert!(generated.contains("public String toString()"));
    assert!(generated.contains("\"Point{\" + \"x=\" + x + \", \" + \"y=\" + y + \"}\""));
}

#[test]
fn to_string_for_with_no_fields_has_no_trailing_concatenation() {
    let generated = to_string_for("Empty", &[], "    ");
    assert!(generated.contains("\"Empty{}\""));
}

#[test]
fn equals_and_hash_code_for_compares_every_field() {
    let generated = equals_and_hash_code_for("Point", &[field("x", "int", false), field("y", "int", false)], "    ");
    assert!(generated.contains("public boolean equals(Object o)"));
    assert!(generated.contains("Point that = (Point) o;"));
    assert!(generated.contains("java.util.Objects.equals(x, that.x) && java.util.Objects.equals(y, that.y)"));
    assert!(generated.contains("public int hashCode()"));
    assert!(generated.contains("java.util.Objects.hash(x, y)"));
}

#[test]
fn equals_and_hash_code_for_with_no_fields_compares_only_class_identity() {
    let generated = equals_and_hash_code_for("Empty", &[], "    ");
    assert!(generated.contains("return true;"));
    assert!(generated.contains("java.util.Objects.hash()"));
}

#[test]
fn generate_dispatches_to_the_right_template() {
    let fields = [field("x", "int", false)];
    let constructor = generate(CodeGeneration::Constructor { type_name: "Foo", fields: &fields }, "    ");
    assert!(constructor.contains("public Foo(int x)"));
    let to_string = generate(CodeGeneration::ToString { type_name: "Foo", fields: &fields }, "    ");
    assert!(to_string.contains("toString()"));
    let equality = generate(CodeGeneration::EqualsAndHashCode { type_name: "Foo", fields: &fields }, "    ");
    assert!(equality.contains("hashCode()"));
}

#[test]
fn default_return_for_void_is_none() {
    assert_eq!(default_return_for("void"), None);
}

#[test]
fn default_return_for_primitives_and_objects() {
    assert_eq!(default_return_for("boolean"), Some("false"));
    assert_eq!(default_return_for("int"), Some("0"));
    assert_eq!(default_return_for("double"), Some("0.0"));
    assert_eq!(default_return_for("String"), Some("null"));
}

#[test]
fn override_stub_for_a_void_method_has_no_return_statement() {
    let stub = override_stub_for(&method("run", "void", vec![]), "    ");
    assert_eq!(stub, "    @Override\n    public void run() {\n    }\n");
}

#[test]
fn override_stub_for_a_method_with_params_and_a_return_type() {
    let stub = override_stub_for(&method("compute", "int", vec![("int", "x")]), "    ");
    assert_eq!(
        stub,
        "    @Override\n    public int compute(int x) {\n        return 0;\n    }\n"
    );
}

#[test]
fn overrides_are_separated_by_a_blank_line() {
    let methods = [method("run", "void", vec![]), method("stop", "void", vec![])];
    let generated = generate(CodeGeneration::Overrides { methods: &methods }, "  ");
    assert_eq!(
        generated,
        "  @Override\n  public void run() {\n  }\n\n  @Override\n  public void stop() {\n  }\n"
    );
}
