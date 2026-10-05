//! The two JVM languages' answers to the editor's type-model questions
//! (`fg_extension::Extension::{enclosing_type, supertype, type_members,
//! receiver_type, types_with_fields}`), assembled from the per-language
//! tree walks in `fields`, `methods`, `kotlin_members` and
//! `identifier_type`.
//!
//! Everything here is a translation layer: those modules speak in
//! `FieldInfo`/`MethodSignature`, the vocabulary they were written in, and
//! this is the one place that turns that into the editor's own
//! `TypeMember`.

use fg_extension::{MemberKind, MemberView, ReceiverType, TypeDeclaration, TypeFields, TypeMember};
use tree_sitter::Tree;

use crate::fields::{FieldInfo, fields_in_type, java_classes_with_fields};
use crate::identifier_type::{
    child_by_kind, enclosing_class_kotlin, type_of_identifier_java, type_of_identifier_kotlin,
};
use crate::kotlin_members::{
    all_kotlin_functions_in_type, kotlin_enclosing_class, kotlin_functions_in_type, kotlin_properties_in_type,
    kotlin_superclass_name,
};
use crate::methods::{MethodSignature, all_methods_in_type, enclosing_class, methods_in_type, superclass_name};
use crate::{JAVA, KOTLIN};

pub fn enclosing_type(language_id: &str, tree: &Tree, source: &str, byte: usize) -> Option<TypeDeclaration> {
    match language_id {
        JAVA => enclosing_class(tree, source, byte).map(|(name, insertion_byte)| TypeDeclaration {
            name,
            insertion_byte: Some(insertion_byte),
        }),
        KOTLIN => {
            let start = tree.root_node().named_descendant_for_byte_range(byte, byte)?;
            let class_node = enclosing_class_kotlin(start)?;
            let name = class_node.child_by_field_name("name")?;
            // A Kotlin class may have no body at all (`class Id(val v: Int)`),
            // and then there is nowhere to put a generated member.
            let insertion_byte = child_by_kind(class_node, "class_body").map(|body| body.end_byte().saturating_sub(1));
            Some(TypeDeclaration {
                name: source[name.byte_range()].to_string(),
                insertion_byte,
            })
        }
        _ => None,
    }
}

pub fn supertype(language_id: &str, tree: &Tree, source: &str, type_name: &str) -> Option<String> {
    match language_id {
        JAVA => superclass_name(tree, source, type_name),
        KOTLIN => kotlin_superclass_name(tree, source, type_name),
        _ => None,
    }
}

/// `type_name`'s own members as `view` sees them.
///
/// Java's `Outside` view lists the methods `methods_in_type` filters to —
/// no `static`, `private` or `final` ones — which is what dot-completion on
/// a value of another type has always offered. Kotlin has no
/// `Overridable` answer: "Override Method" generates Java, so nobody asks.
pub fn type_members(language_id: &str, tree: &Tree, source: &str, type_name: &str, view: MemberView) -> Vec<TypeMember> {
    let (fields, methods) = match (language_id, view) {
        (JAVA, MemberView::Inside) => (
            fields_in_type(tree, source, type_name, true),
            all_methods_in_type(tree, source, type_name),
        ),
        (JAVA, MemberView::Outside) => (
            fields_in_type(tree, source, type_name, false),
            methods_in_type(tree, source, type_name),
        ),
        (JAVA, MemberView::Overridable) => (Vec::new(), methods_in_type(tree, source, type_name)),
        (KOTLIN, MemberView::Inside) => (
            kotlin_properties_in_type(tree, source, type_name),
            all_kotlin_functions_in_type(tree, source, type_name),
        ),
        (KOTLIN, MemberView::Outside) => (
            kotlin_properties_in_type(tree, source, type_name),
            kotlin_functions_in_type(tree, source, type_name),
        ),
        _ => return Vec::new(),
    };
    fields
        .into_iter()
        .map(field_member)
        .chain(methods.into_iter().map(method_member))
        .collect()
}

/// `this` is the type the cursor sits in, `super` the one it extends, and
/// any other word a variable whose declared type is looked up — parameter,
/// then local, then field. Outside any type nothing resolves, `this` or
/// not.
pub fn receiver_type(language_id: &str, tree: &Tree, source: &str, byte: usize, receiver: &str) -> Option<ReceiverType> {
    let class_name = match language_id {
        JAVA => enclosing_class(tree, source, byte)?.0,
        KOTLIN => kotlin_enclosing_class(tree, source, byte)?,
        _ => return None,
    };
    let (type_name, view, in_this_file) = match receiver {
        "this" => (class_name, MemberView::Inside, true),
        "super" => (supertype(language_id, tree, source, &class_name)?, MemberView::Inside, false),
        name => (identifier_type(language_id, tree, source, byte, name)?, MemberView::Outside, false),
    };
    Some(ReceiverType {
        type_name,
        view,
        in_this_file,
    })
}

/// A variable's declared type at `byte`: parameter, then local, then field.
fn identifier_type(language_id: &str, tree: &Tree, source: &str, byte: usize, name: &str) -> Option<String> {
    match language_id {
        JAVA => type_of_identifier_java(tree, source, byte, name),
        KOTLIN => type_of_identifier_kotlin(tree, source, byte, name),
        _ => None,
    }
}

/// Java's classes with instance fields. Kotlin has none to offer: code
/// generation is Java-only (see `codegen`).
pub fn types_with_fields(language_id: &str, tree: &Tree, source: &str) -> Vec<TypeFields> {
    if language_id != JAVA {
        return Vec::new();
    }
    java_classes_with_fields(tree, source)
        .into_iter()
        .map(|class| TypeFields {
            declaration: TypeDeclaration {
                name: class.name,
                insertion_byte: Some(class.insertion_byte),
            },
            fields: class.fields.into_iter().map(field_member).collect(),
        })
        .collect()
}

fn field_member(field: FieldInfo) -> TypeMember {
    TypeMember {
        name: field.name,
        kind: MemberKind::Field,
        type_text: field.java_type,
        params: Vec::new(),
        read_only: field.is_final,
    }
}

fn method_member(method: MethodSignature) -> TypeMember {
    TypeMember {
        name: method.name,
        kind: MemberKind::Method,
        type_text: method.return_type,
        params: method.params,
        read_only: false,
    }
}

#[cfg(test)]
#[path = "code_model_test.rs"]
mod code_model_test;
