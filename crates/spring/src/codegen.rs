//! Java code generation — accessors, a field-assigning constructor,
//! `toString`, `equals`/`hashCode` and `@Override` stubs. Text only: the
//! editor decides which fields and methods (its pickers), and where the
//! block goes (just before the type body's closing `}`, which
//! `TypeDeclaration::insertion_byte` names).
//!
//! Java only. Kotlin's `val`/`var` properties already *are* getters and
//! setters, and a `data class` already has `toString`/`equals`/`hashCode`,
//! so generating explicit ones there isn't the idiomatic move the Java
//! boilerplate commands are.

use fg_extension::{CodeGeneration, TypeMember};

/// The block `request` asks for, every line indented with `indent_unit`.
pub fn generate(request: CodeGeneration<'_>, indent_unit: &str) -> String {
    match request {
        CodeGeneration::Accessors {
            fields,
            getters,
            setters,
        } => accessors(fields, indent_unit, getters, setters),
        CodeGeneration::Constructor { type_name, fields } => constructor_for(type_name, fields, indent_unit),
        CodeGeneration::ToString { type_name, fields } => to_string_for(type_name, fields, indent_unit),
        CodeGeneration::EqualsAndHashCode { type_name, fields } => {
            equals_and_hash_code_for(type_name, fields, indent_unit)
        }
        CodeGeneration::Overrides { methods } => methods
            .iter()
            .map(|method| override_stub_for(method, indent_unit))
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

/// Uppercases the first character of `name` for the `getX`/`setX` accessor
/// method name suffix, leaving the rest as-is (so e.g. `userId` becomes
/// `UserId`, matching standard Java bean-accessor naming).
fn capitalized(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn getter_for(field: &TypeMember, indent_unit: &str) -> String {
    let cap = capitalized(&field.name);
    let ty = &field.type_text;
    let name = &field.name;
    format!(
        "{indent_unit}public {ty} get{cap}() {{\n\
         {indent_unit}{indent_unit}return this.{name};\n\
         {indent_unit}}}\n"
    )
}

/// `None` for a `final` field — it can't be reassigned, so a setter for it
/// wouldn't compile.
fn setter_for(field: &TypeMember, indent_unit: &str) -> Option<String> {
    if field.read_only {
        return None;
    }
    let cap = capitalized(&field.name);
    let ty = &field.type_text;
    let name = &field.name;
    Some(format!(
        "{indent_unit}public void set{cap}({ty} {name}) {{\n\
         {indent_unit}{indent_unit}this.{name} = {name};\n\
         {indent_unit}}}\n"
    ))
}

/// The accessors asked for, for `field`. With both, the getter comes first,
/// a blank line, then the setter (skipped for a `final` field) — standard
/// Java accessor shape, `this.` on the getter's return and the setter's
/// assignment to disambiguate the field from the setter's identically-named
/// parameter. Empty for setters alone on a `final` field — there's nothing
/// to generate.
fn accessors_for(field: &TypeMember, indent_unit: &str, getters: bool, setters: bool) -> String {
    let mut out = String::new();
    if getters {
        out.push_str(&getter_for(field, indent_unit));
    }
    if setters && let Some(setter) = setter_for(field, indent_unit) {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&setter);
    }
    out
}

/// The accessors asked for, for every field in `fields`, each field's block
/// separated by a blank line. Empty if `fields` is empty, or if only setters
/// were asked for and every field is `final`.
fn accessors(fields: &[TypeMember], indent_unit: &str, getters: bool, setters: bool) -> String {
    fields
        .iter()
        .map(|field| accessors_for(field, indent_unit, getters, setters))
        .filter(|block| !block.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// A constructor assigning every field from a same-named parameter —
/// `this.x = x;` per field, the same disambiguation `setter_for` already
/// uses. Still valid Java with zero fields (an empty, parameterless
/// constructor), so this doesn't special-case that.
fn constructor_for(class_name: &str, fields: &[TypeMember], indent_unit: &str) -> String {
    let params = fields
        .iter()
        .map(|f| format!("{} {}", f.type_text, f.name))
        .collect::<Vec<_>>()
        .join(", ");
    let assignments: String = fields
        .iter()
        .map(|f| format!("{indent_unit}{indent_unit}this.{name} = {name};\n", name = f.name))
        .collect();
    format!("{indent_unit}public {class_name}({params}) {{\n{assignments}{indent_unit}}}\n")
}

/// `@Override public String toString()`, string-concatenation form (no
/// import needed, unlike `String.format`/text blocks) — `"ClassName{x=" +
/// x + ", y=" + y + "}"`.
fn to_string_for(class_name: &str, fields: &[TypeMember], indent_unit: &str) -> String {
    let body = if fields.is_empty() {
        format!("\"{class_name}{{}}\"")
    } else {
        let parts = fields
            .iter()
            .map(|f| format!("\"{name}=\" + {name}", name = f.name))
            .collect::<Vec<_>>()
            .join(" + \", \" + ");
        format!("\"{class_name}{{\" + {parts} + \"}}\"")
    };
    format!(
        "{indent_unit}@Override\n\
         {indent_unit}public String toString() {{\n\
         {indent_unit}{indent_unit}return {body};\n\
         {indent_unit}}}\n"
    )
}

/// `@Override public boolean equals(Object o)` + `@Override public int
/// hashCode()`, generated together (standard IDE behavior — the two must
/// stay consistent with each other, so there's no separate "just equals"/
/// "just hashCode" option the way getters/setters have). Uses
/// `java.util.Objects.equals`/`.hash` (fully qualified, deliberately —
/// correct for primitives via autoboxing same as for objects, and avoids
/// needing to check for or insert an `import java.util.Objects;` line the
/// way a bare `Objects.equals(...)` call would need). A zero-field class
/// still generates validly: `equals` reduces to comparing only class
/// identity, `hashCode` to `Objects.hash()` (a constant).
fn equals_and_hash_code_for(class_name: &str, fields: &[TypeMember], indent_unit: &str) -> String {
    let comparison = if fields.is_empty() {
        format!("{indent_unit}{indent_unit}return true;\n")
    } else {
        let conditions = fields
            .iter()
            .map(|f| format!("java.util.Objects.equals({name}, that.{name})", name = f.name))
            .collect::<Vec<_>>()
            .join(" && ");
        format!("{indent_unit}{indent_unit}return {conditions};\n")
    };
    let equals = format!(
        "{indent_unit}@Override\n\
         {indent_unit}public boolean equals(Object o) {{\n\
         {indent_unit}{indent_unit}if (this == o) return true;\n\
         {indent_unit}{indent_unit}if (o == null || getClass() != o.getClass()) return false;\n\
         {indent_unit}{indent_unit}{class_name} that = ({class_name}) o;\n\
         {comparison}\
         {indent_unit}}}\n"
    );

    let hash_args = fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(", ");
    let hash_code = format!(
        "{indent_unit}@Override\n\
         {indent_unit}public int hashCode() {{\n\
         {indent_unit}{indent_unit}return java.util.Objects.hash({hash_args});\n\
         {indent_unit}}}\n"
    );

    format!("{equals}\n{hash_code}")
}

/// The literal Java expression an override stub should `return` for
/// `java_type` — `None` for `void`, which needs no `return` statement at
/// all. Every non-`void` type needs *something*, since a stub with a
/// missing return statement wouldn't compile.
fn default_return_for(java_type: &str) -> Option<&'static str> {
    match java_type {
        "void" => None,
        "boolean" => Some("false"),
        "byte" | "short" | "int" | "long" => Some("0"),
        "float" => Some("0.0f"),
        "double" => Some("0.0"),
        "char" => Some("'\\0'"),
        _ => Some("null"),
    }
}

/// `@Override` plus a stub body returning `default_return_for`'s value (or
/// no `return` at all, for `void`) — one inherited method turned into a
/// compilable override.
fn override_stub_for(method: &TypeMember, indent_unit: &str) -> String {
    let params = method
        .params
        .iter()
        .map(|(ty, name)| format!("{ty} {name}"))
        .collect::<Vec<_>>()
        .join(", ");
    let body = match default_return_for(&method.type_text) {
        Some(value) => format!("{indent_unit}{indent_unit}return {value};\n"),
        None => String::new(),
    };
    let ty = &method.type_text;
    let name = &method.name;
    format!("{indent_unit}@Override\n{indent_unit}public {ty} {name}({params}) {{\n{body}{indent_unit}}}\n")
}

#[cfg(test)]
#[path = "codegen_test.rs"]
mod codegen_test;
