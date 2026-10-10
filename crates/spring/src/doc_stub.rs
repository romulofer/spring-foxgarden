//! A Javadoc (Java) or KDoc (Kotlin) skeleton for the declaration under the
//! cursor.
//!
//! Java gets `@param` per parameter, `@return` unless the method is `void`,
//! `@throws` per declared exception, plus `@param <T>` for type parameters.
//! Kotlin gets `@param` per parameter, `@property` for a primary-constructor
//! `val`/`var`, `@param T` per type parameter, and `@return` for a function
//! that returns something. Kotlin has no checked exceptions, so no `@throws`.
//!
//! Syntactic only, like the rest of the type model: it reads the tree the
//! editor already parsed. It cannot tell a checked Java exception from an
//! unchecked one, so it lists every one the declaration names — a stub is
//! something to prune, and a missing `@throws` is worse than an extra one.

use fg_extension::DocStub;
use tree_sitter::{Node, Tree};

use crate::{JAVA, KOTLIN};

/// Declarations a Javadoc comment can describe.
const JAVA_DOCUMENTABLE: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
    "record_declaration",
    "annotation_type_declaration",
    "method_declaration",
    "constructor_declaration",
];

/// Declarations a KDoc comment can describe. An interface and an enum are
/// both `class_declaration` in this grammar.
const KOTLIN_DOCUMENTABLE: &[&str] = &[
    "class_declaration",
    "object_declaration",
    "companion_object",
    "function_declaration",
    "secondary_constructor",
];

/// The skeleton for the innermost documentable declaration containing
/// `byte`, or `None` when there is none or it already has a `/** */`.
pub fn doc_stub(tree: &Tree, source: &str, language_id: &str, byte: usize) -> Option<DocStub> {
    let documentable = match language_id {
        JAVA => JAVA_DOCUMENTABLE,
        KOTLIN => KOTLIN_DOCUMENTABLE,
        _ => return None,
    };
    let byte = byte.min(source.len());
    // A caret at the very end of a declaration (`fun f() = x|`) sits just
    // outside it as far as the tree is concerned, so look one byte back too.
    let node = [byte, byte.saturating_sub(1)].into_iter().find_map(|at| {
        let mut node = tree.root_node().descendant_for_byte_range(at, at)?;
        while !documentable.contains(&node.kind()) {
            node = node.parent()?;
        }
        Some(node)
    })?;
    if has_doc_comment(node, source) {
        return None;
    }

    let start = node.start_byte();
    let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    let before = &source[line_start..start];
    let indent: String = before.chars().take_while(|c| c.is_whitespace()).collect();

    let tags = match language_id {
        JAVA => java_tags(node, source),
        _ => kotlin_tags(node, source),
    };
    let mut lines = vec!["/**".to_string(), " * ".to_string()];
    if !tags.is_empty() {
        lines.push(" *".to_string());
        lines.extend(tags.into_iter().map(|tag| format!(" * {tag}")));
    }
    lines.push(" */".to_string());
    let comment = lines
        .iter()
        .map(|line| format!("{indent}{line}"))
        .collect::<Vec<_>>()
        .join("\n");

    if before.trim().is_empty() {
        // The declaration starts its line: the comment takes the lines above.
        Some(DocStub {
            byte: line_start,
            text: format!("{comment}\n"),
        })
    } else {
        // Something precedes it on the line (`} class B {`): keep that where
        // it is and put the comment right before the declaration.
        Some(DocStub {
            byte: start,
            text: format!("{}\n{indent}", comment.trim_start()),
        })
    }
}

/// Whether the previous sibling is a `/** */` comment — the one Javadoc
/// attaches to. A plain `/* */` or `//` comment does not count.
fn has_doc_comment(node: Node<'_>, source: &str) -> bool {
    node.prev_sibling().is_some_and(|prev| {
        matches!(prev.kind(), "block_comment") && source[prev.byte_range()].starts_with("/**")
    })
}

/// The `@param`/`@return`/`@throws` lines, in the order Javadoc conventions
/// list them.
fn java_tags(node: Node<'_>, source: &str) -> Vec<String> {
    let text = |n: Node<'_>| source[n.byte_range()].to_string();
    let mut tags = Vec::new();

    if let Some(params) = node.child_by_field_name("type_parameters") {
        let mut cursor = params.walk();
        for param in params.named_children(&mut cursor).filter(|c| c.kind() == "type_parameter") {
            let mut inner = param.walk();
            if let Some(name) = param.named_children(&mut inner).find(|c| c.kind() == "type_identifier") {
                tags.push(format!("@param <{}>", text(name)));
            }
        }
    }

    if let Some(params) = node.child_by_field_name("parameters") {
        let mut cursor = params.walk();
        for param in params.named_children(&mut cursor) {
            let name = match param.kind() {
                "formal_parameter" | "receiver_parameter" => param.child_by_field_name("name"),
                // `String... names`: the name sits inside a declarator.
                "spread_parameter" => {
                    let mut inner = param.walk();
                    param
                        .named_children(&mut inner)
                        .find(|c| c.kind() == "variable_declarator")
                        .and_then(|d| d.child_by_field_name("name"))
                }
                _ => None,
            };
            if let Some(name) = name {
                tags.push(format!("@param {}", text(name)));
            }
        }
    }

    if node.kind() == "method_declaration"
        && node.child_by_field_name("type").is_some_and(|ty| text(ty) != "void")
    {
        tags.push("@return".to_string());
    }

    let mut cursor = node.walk();
    if let Some(throws) = node.children(&mut cursor).find(|c| c.kind() == "throws") {
        let mut inner = throws.walk();
        for exception in throws.named_children(&mut inner) {
            tags.push(format!("@throws {}", text(exception)));
        }
    }
    tags
}

/// The `@param`/`@property`/`@return` lines of a Kotlin declaration.
fn kotlin_tags(node: Node<'_>, source: &str) -> Vec<String> {
    let text = |n: Node<'_>| source[n.byte_range()].to_string();
    let mut tags = Vec::new();
    let mut cursor = node.walk();
    let children: Vec<Node<'_>> = node.children(&mut cursor).collect();

    if let Some(params) = children.iter().find(|c| c.kind() == "type_parameters") {
        let mut inner = params.walk();
        for param in params.named_children(&mut inner).filter(|c| c.kind() == "type_parameter") {
            if let Some(name) = first_identifier(param) {
                tags.push(format!("@param {}", text(name)));
            }
        }
    }

    // Parameters of a function or secondary constructor, or of the class's
    // primary constructor, where a `val`/`var` declares a property.
    let primary = children
        .iter()
        .find(|c| c.kind() == "primary_constructor")
        .and_then(|c| c.named_child(0));
    let lists = children
        .iter()
        .find(|c| c.kind() == "function_value_parameters")
        .copied()
        .into_iter()
        .chain(primary);
    for list in lists {
        let mut inner = list.walk();
        for param in list
            .named_children(&mut inner)
            .filter(|c| matches!(c.kind(), "parameter" | "class_parameter"))
        {
            let Some(name) = first_identifier(param) else { continue };
            let mut tokens = param.walk();
            let declares_property = param.children(&mut tokens).any(|t| matches!(t.kind(), "val" | "var"));
            let tag = if declares_property { "@property" } else { "@param" };
            tags.push(format!("{tag} {}", text(name)));
        }
    }

    if matches!(node.kind(), "function_declaration") && returns_a_value(&children, source) {
        tags.push("@return".to_string());
    }
    tags
}

/// The first `identifier` directly under `node` — a parameter's or type
/// parameter's name, which comes before its type.
fn first_identifier(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).find(|c| c.kind() == "identifier")
}

/// Whether a Kotlin function returns something: an explicit return type other
/// than `Unit`, or — with no declared type — an expression body (`= value`)
/// rather than a block.
fn returns_a_value(children: &[Node<'_>], source: &str) -> bool {
    let after_params = children
        .iter()
        .skip_while(|c| c.kind() != "function_value_parameters")
        .skip(1);
    let mut declared_type = None;
    let mut body = None;
    for child in after_params {
        match child.kind() {
            "function_body" => body = Some(*child),
            kind if kind.ends_with("type") => declared_type = Some(*child),
            _ => {}
        }
    }
    match (declared_type, body) {
        (Some(ty), _) => &source[ty.byte_range()] != "Unit",
        (None, Some(body)) => body.named_child(0).is_some_and(|first| first.kind() != "block"),
        (None, None) => false,
    }
}

#[cfg(test)]
#[path = "doc_stub_test.rs"]
mod doc_stub_test;
