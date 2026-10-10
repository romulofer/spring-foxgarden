//! A Javadoc skeleton for the declaration under the cursor: `@param` per
//! parameter, `@return` unless the method is `void`, `@throws` per declared
//! exception, plus `@param <T>` for type parameters.
//!
//! Syntactic only, like the rest of the type model: it reads the tree the
//! editor already parsed. It cannot tell a checked exception from an
//! unchecked one, so it lists every one the declaration names — a stub is
//! something to prune, and a missing `@throws` is worse than an extra one.

use fg_extension::DocStub;
use tree_sitter::{Node, Tree};

use crate::JAVA;

/// Declarations a Javadoc comment can describe.
const DOCUMENTABLE: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "enum_declaration",
    "record_declaration",
    "annotation_type_declaration",
    "method_declaration",
    "constructor_declaration",
];

/// The skeleton for the innermost documentable declaration containing
/// `byte`, or `None` when there is none or it already has a `/** */`.
pub fn doc_stub(tree: &Tree, source: &str, language_id: &str, byte: usize) -> Option<DocStub> {
    if language_id != JAVA {
        return None;
    }
    let byte = byte.min(source.len());
    let mut node = tree.root_node().descendant_for_byte_range(byte, byte)?;
    while !DOCUMENTABLE.contains(&node.kind()) {
        node = node.parent()?;
    }
    if has_doc_comment(node, source) {
        return None;
    }

    let start = node.start_byte();
    let line_start = source[..start].rfind('\n').map_or(0, |i| i + 1);
    let before = &source[line_start..start];
    let indent: String = before.chars().take_while(|c| c.is_whitespace()).collect();

    let tags = tags(node, source);
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
fn tags(node: Node<'_>, source: &str) -> Vec<String> {
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

#[cfg(test)]
#[path = "javadoc_test.rs"]
mod javadoc_test;
