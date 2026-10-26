//! Finding the tests in a file — the JUnit/TestNG methods and the classes
//! holding them — for the editor's run gutter, which puts a ▶ beside each.
//!
//! Syntactic like [`crate::main_entry`], for the same reason: the marker has to
//! be there the instant a file opens, from the tree the editor already
//! parsed. A test is a method annotated `@Test` (or one of JUnit 5's other
//! test annotations), found by the annotation's name, not resolved: a
//! `@Test` from an unrelated library is a false marker the build panel then
//! reports as having run no tests, which is cheaper than asking a language
//! server for every file.

use fg_extension::{RunKind, RunTarget};
use tree_sitter::{Node, Tree};

use crate::main_entry::{enclosing_type_chain, package_name, qualified, walk};
use crate::{JAVA, KOTLIN};

/// Annotations that make a method a test, by simple name.
const TEST_ANNOTATIONS: [&str; 5] = ["Test", "ParameterizedTest", "RepeatedTest", "TestFactory", "TestTemplate"];

/// Every test method in `source`, and every class directly declaring one, in
/// source order: the class's marker runs all its tests, a method's runs that one.
pub fn test_targets(tree: &Tree, source: &str, language_id: &str) -> Vec<RunTarget> {
    let (package_kind, method_kind, class_kinds): (&str, &str, &[&str]) = match language_id {
        JAVA => ("package_declaration", "method_declaration", &["class_declaration"]),
        KOTLIN => ("package_header", "function_declaration", &["class_declaration"]),
        _ => return Vec::new(),
    };
    let package = package_name(tree, source, package_kind);
    let mut out: Vec<RunTarget> = Vec::new();
    walk(tree.root_node(), &mut |node| {
        if node.kind() != method_kind || !is_test(node, source) {
            return;
        }
        let Some(method) = node.child_by_field_name("name") else { return };
        let Some(class) = enclosing_type_chain(node, source, class_kinds) else { return };
        let class_entry = qualified(package.as_ref(), &class);
        if let Some(class_node) = enclosing_class(node, class_kinds)
            && let Some(name) = class_node.child_by_field_name("name")
        {
            let line = name.start_position().row;
            if !out.iter().any(|t| t.entry_point == class_entry) {
                out.push(RunTarget {
                    line,
                    kind: RunKind::Test,
                    entry_point: class_entry.clone(),
                    label: class.rsplit('$').next().unwrap_or(&class).to_string(),
                });
            }
        }
        let name = source[method.byte_range()].trim_matches('`').to_string();
        out.push(RunTarget {
            line: method.start_position().row,
            kind: RunKind::SingleTest,
            entry_point: format!("{class_entry}#{name}"),
            label: name,
        });
    });
    out.sort_by_key(|target| target.line);
    out
}

fn enclosing_class<'a>(node: Node<'a>, kinds: &[&str]) -> Option<Node<'a>> {
    let mut current = node.parent();
    while let Some(parent) = current {
        if kinds.contains(&parent.kind()) {
            return Some(parent);
        }
        current = parent.parent();
    }
    None
}

/// Whether `method` carries a test annotation. Read off the modifiers' text
/// so Java's `marker_annotation`/`annotation` and Kotlin's `annotation` need
/// no separate handling.
fn is_test(method: Node, source: &str) -> bool {
    let mut cursor = method.walk();
    method
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "modifiers")
        .flat_map(|modifiers| {
            let mut inner = modifiers.walk();
            modifiers.named_children(&mut inner).collect::<Vec<_>>()
        })
        .filter(|child| child.kind().contains("annotation"))
        .any(|annotation| TEST_ANNOTATIONS.contains(&annotation_name(&source[annotation.byte_range()])))
}

/// `Test` from `@Test`, `@org.junit.Test` or `@Test(expected = X.class)`.
fn annotation_name(text: &str) -> &str {
    let text = text.trim().trim_start_matches('@');
    let name = text.split(['(', ' ', '\t', '\n']).next().unwrap_or(text);
    name.rsplit('.').next().unwrap_or(name)
}

#[cfg(test)]
#[path = "test_entry_test.rs"]
mod test_entry_test;
