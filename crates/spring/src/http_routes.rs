//! Spring MVC's HTTP routes — the `@GetMapping`/`@RequestMapping` family
//! on controller methods, joined with the class's own `@RequestMapping`
//! base path, in both JVM languages. Feeds the editor's route map
//! (`Extension::http_routes`), which lists every route in a project and
//! jumps to its handler.
//!
//! Syntactic, like `main_entry`: it reads the tree the editor parsed, so
//! the map works before any language server is up, and the annotations'
//! simple names are matched rather than their resolved types — the same
//! "match the simple name, not the qualified path" limitation the rest of
//! this extension accepts.

use fg_extension::HttpRoute;
use tree_sitter::{Node, Tree};

use crate::{JAVA, KOTLIN};

/// Every route declared in `source`, in source order. Nothing for a
/// language this extension does not find routes in.
pub fn http_routes(tree: &Tree, source: &str, language_id: &str) -> Vec<HttpRoute> {
    match language_id {
        JAVA => java_routes(tree, source),
        KOTLIN => kotlin_routes(tree, source),
        _ => Vec::new(),
    }
}

fn child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor).find(|c| c.kind() == kind)
}

/// Recognized mapping annotation names and the HTTP method each fixes, or
/// `None` for `@RequestMapping`, whose method comes from its own `method =`
/// element (defaulting to `"ANY"`) rather than the annotation name.
fn fixed_http_method(annotation_name: &str) -> Option<Option<&'static str>> {
    match annotation_name {
        "GetMapping" => Some(Some("GET")),
        "PostMapping" => Some(Some("POST")),
        "PutMapping" => Some(Some("PUT")),
        "DeleteMapping" => Some(Some("DELETE")),
        "PatchMapping" => Some(Some("PATCH")),
        "RequestMapping" => Some(None),
        _ => None,
    }
}

/// The simple name of an `annotation`/`marker_annotation` node — ignores the
/// rarer fully-qualified `@org.springframework....GetMapping` shape
/// (`scoped_identifier`), same "match the simple name, not the qualified
/// path" limitation `SPEC.md` §0/§2 accepts.
fn annotation_name(node: Node, source: &str) -> Option<String> {
    let name = node.child_by_field_name("name")?;
    if name.kind() != "identifier" {
        return None;
    }
    Some(source[name.byte_range()].to_string())
}

/// A `string_literal`'s actual text — its `string_fragment` child, not its
/// own span (which includes the quotes). An empty literal (`""`) has no
/// `string_fragment` child at all, hence the empty-string fallback.
fn string_literal_text(node: Node, source: &str) -> String {
    node.named_child(0)
        .map(|fragment| source[fragment.byte_range()].to_string())
        .unwrap_or_default()
}

/// A path value as Java writes it: a `string_literal`, or an array
/// initializer (`{"/users"}`, which Spring accepts anywhere a single path
/// goes), whose first path is taken — the same choice the Kotlin side makes
/// for a collection literal.
fn path_value(node: Node, source: &str) -> Option<String> {
    match node.kind() {
        "string_literal" => Some(string_literal_text(node, source)),
        "element_value_array_initializer" => {
            let mut cursor = node.walk();
            let first = node.named_children(&mut cursor).find(|c| c.kind() == "string_literal");
            first.map(|literal| string_literal_text(literal, source))
        }
        _ => None,
    }
}

/// The path an `annotation`/`marker_annotation` node contributes: a bare
/// positional path value, a `value =`/`path =` `element_value_pair`
/// (Spring accepts either key as a synonym), or `""` for a marker
/// annotation (no `()` at all) or an annotation with neither shape present.
fn annotation_path(node: Node, source: &str) -> String {
    let Some(args) = node.child_by_field_name("arguments") else {
        return String::new();
    };
    let mut cursor = args.walk();
    for child in args.named_children(&mut cursor) {
        match child.kind() {
            "string_literal" | "element_value_array_initializer" => {
                return path_value(child, source).unwrap_or_default();
            }
            "element_value_pair" => {
                let Some(key) = child.child_by_field_name("key") else {
                    continue;
                };
                let key_text = &source[key.byte_range()];
                if key_text != "value" && key_text != "path" {
                    continue;
                }
                if let Some(path) = child
                    .child_by_field_name("value")
                    .and_then(|value| path_value(value, source))
                {
                    return path;
                }
            }
            _ => {}
        }
    }
    String::new()
}

/// `@RequestMapping(method = RequestMethod.DELETE)`'s HTTP method — a
/// `field_access` (`RequestMethod` `.` `DELETE`); take the identifier after
/// the dot. `None` (defaulting to `"ANY"`) if there's no `method =` element.
fn annotation_method_override(node: Node, source: &str) -> Option<String> {
    let args = node.child_by_field_name("arguments")?;
    let mut cursor = args.walk();
    for child in args.named_children(&mut cursor) {
        if child.kind() != "element_value_pair" {
            continue;
        }
        let key = child.child_by_field_name("key")?;
        if &source[key.byte_range()] != "method" {
            continue;
        }
        let value = child.child_by_field_name("value")?;
        if value.kind() != "field_access" {
            continue;
        }
        let field = value.child_by_field_name("field")?;
        return Some(source[field.byte_range()].to_string());
    }
    None
}

/// Joins a class-level base path with a method-level path: strips a
/// trailing `/` off the base, ensures a leading `/` on the method path (if
/// it's non-empty and missing one), concatenates. Both sides empty resolves
/// to `"/"` rather than `""`.
fn join_paths(base: &str, method_path: &str) -> String {
    let base = base.trim_end_matches('/');
    let method = if method_path.is_empty() {
        String::new()
    } else if let Some(stripped) = method_path.strip_prefix('/') {
        format!("/{stripped}")
    } else {
        format!("/{method_path}")
    };
    let joined = format!("{base}{method}");
    if joined.is_empty() { "/".to_string() } else { joined }
}

/// The class's own `@RequestMapping` base path, if its `modifiers` child
/// carries one — `@RestController`/`@Controller` alone contribute no path.
fn class_base_path(class_node: Node, source: &str) -> String {
    let Some(modifiers) = child_by_kind(class_node, "modifiers") else {
        return String::new();
    };
    let mut cursor = modifiers.walk();
    for annotation in modifiers.named_children(&mut cursor) {
        if !matches!(annotation.kind(), "annotation" | "marker_annotation") {
            continue;
        }
        if annotation_name(annotation, source).as_deref() == Some("RequestMapping") {
            return annotation_path(annotation, source);
        }
    }
    String::new()
}

/// `(http_method, path)` from the first recognized mapping annotation on
/// `method_node`'s `modifiers` child, if any — a method only becomes an
/// endpoint if it carries one of the five recognized annotations,
/// independent of whether the enclosing class carries
/// `@Controller`/`@RestController` at all. Other annotations on the same
/// node (`@Override`, `@Transactional`, …) are ignored, not an error.
fn method_mapping(method_node: Node, source: &str) -> Option<(String, String)> {
    let modifiers = child_by_kind(method_node, "modifiers")?;
    let mut cursor = modifiers.walk();
    for annotation in modifiers.named_children(&mut cursor) {
        if !matches!(annotation.kind(), "annotation" | "marker_annotation") {
            continue;
        }
        let Some(name) = annotation_name(annotation, source) else {
            continue;
        };
        let Some(fixed) = fixed_http_method(&name) else {
            continue;
        };
        let http_method = match fixed {
            Some(method) => method.to_string(),
            None => annotation_method_override(annotation, source).unwrap_or_else(|| "ANY".to_string()),
        };
        let path = annotation_path(annotation, source);
        return Some((http_method, path));
    }
    None
}

/// Every Spring MVC endpoint declared in `tree` — walks every
/// `class_declaration` (top-level and nested, same "don't miss nested
/// classes" precedent `fields.rs`/`methods.rs` already set), reading each
/// one's own `@RequestMapping` (if any) as a base path, then every
/// `method_declaration` in its body carrying a recognized mapping
/// annotation.
fn java_routes(tree: &Tree, source: &str) -> Vec<HttpRoute> {
    let mut out = Vec::new();
    collect_java_routes(tree.root_node(), source, &mut out);
    out
}

fn collect_java_routes(node: Node, source: &str, out: &mut Vec<HttpRoute>) {
    if node.kind() == "class_declaration"
        && let Some(name_node) = node.child_by_field_name("name")
        && let Some(body) = node.child_by_field_name("body")
    {
        let controller_name = source[name_node.byte_range()].to_string();
        let base_path = class_base_path(node, source);

        let mut cursor = body.walk();
        for member in body.children(&mut cursor) {
            if member.kind() != "method_declaration" {
                continue;
            }
            let Some((http_method, method_path)) = method_mapping(member, source) else {
                continue;
            };
            let Some(handler_name_node) = member.child_by_field_name("name") else {
                continue;
            };
            out.push(HttpRoute {
                method: http_method,
                path: join_paths(&base_path, &method_path),
                owner: controller_name.clone(),
                handler: source[handler_name_node.byte_range()].to_string(),
                handler_byte: handler_name_node.start_byte(),
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_java_routes(child, source, out);
    }
}

/// The `identifier` child of a `user_type` node (`Foo` in `@Foo`/`Foo()`) —
/// ignores any `type_arguments`/`type_modifiers` siblings, same "match the
/// simple name" limitation the Java side accepts.
fn kotlin_user_type_identifier(user_type: Node, source: &str) -> Option<String> {
    let mut cursor = user_type.walk();
    user_type
        .named_children(&mut cursor)
        .find(|c| c.kind() == "identifier")
        .map(|ident| source[ident.byte_range()].to_string())
}

/// An `annotation` node's own name: its one child is either a
/// `constructor_invocation` (`@GetMapping("/x")`, whose own first child is
/// the `user_type`) or a bare `user_type` directly (a marker annotation
/// with no args, e.g. `@PostMapping`) — verified fresh against
/// `tree-sitter-kotlin-ng` 1.1.0's real parse output, not assumed.
fn kotlin_annotation_name(node: Node, source: &str) -> Option<String> {
    let inner = node.named_child(0)?;
    let user_type = match inner.kind() {
        "constructor_invocation" => inner.named_child(0)?,
        "user_type" => inner,
        _ => return None,
    };
    kotlin_user_type_identifier(user_type, source)
}

/// A `string_literal`'s actual text — its `string_content` child (Kotlin's
/// own name for what Java calls `string_fragment`, a different node kind,
/// same role).
fn kotlin_string_literal_text(node: Node, source: &str) -> String {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .find(|c| c.kind() == "string_content")
        .map(|content| source[content.byte_range()].to_string())
        .unwrap_or_default()
}

/// A `value_argument`'s `(key, value)` parts, by position — this grammar
/// defines no fields on `value_argument`, verified against its own
/// `node-types.json` (an `identifier` then the value expression for
/// `key = value`; the value expression alone for a bare positional
/// argument), unlike Java's `element_value_pair`'s `key`/`value` fields.
fn kotlin_value_argument_parts(arg: Node) -> Option<(Option<Node>, Node)> {
    let mut cursor = arg.walk();
    let children: Vec<Node> = arg.named_children(&mut cursor).collect();
    match children.len() {
        1 => Some((None, children[0])),
        2 => Some((Some(children[0]), children[1])),
        _ => None,
    }
}

/// A `value_argument`'s value as a path string: a bare `string_literal`, or
/// — Kotlin requires an array literal where Java accepts a bare value for
/// a single-element case (`method = [RequestMethod.DELETE]`,
/// `path = ["/x"]`) — a `collection_literal` wrapping one, unwrapped to its
/// first element (same one-value scope limit the Java side already
/// accepts for `method =`).
fn kotlin_string_from_value(value: Node, source: &str) -> Option<String> {
    match value.kind() {
        "string_literal" => Some(kotlin_string_literal_text(value, source)),
        "collection_literal" => value
            .named_child(0)
            .filter(|c| c.kind() == "string_literal")
            .map(|lit| kotlin_string_literal_text(lit, source)),
        _ => None,
    }
}

/// The path an `annotation` node contributes: a bare positional
/// `string_literal` argument, a `value =`/`path =` argument (Spring accepts
/// either key as a synonym), or `""` for a marker annotation (no
/// `constructor_invocation` at all) or an annotation with neither shape
/// present.
fn kotlin_annotation_path(node: Node, source: &str) -> String {
    let Some(inner) = node.named_child(0) else {
        return String::new();
    };
    if inner.kind() != "constructor_invocation" {
        return String::new();
    }
    let Some(value_arguments) = child_by_kind(inner, "value_arguments") else {
        return String::new();
    };
    let mut cursor = value_arguments.walk();
    for arg in value_arguments.named_children(&mut cursor) {
        if arg.kind() != "value_argument" {
            continue;
        }
        let Some((key, value)) = kotlin_value_argument_parts(arg) else {
            continue;
        };
        match key {
            None => {
                if let Some(text) = kotlin_string_from_value(value, source) {
                    return text;
                }
            }
            Some(key_node) => {
                let key_text = &source[key_node.byte_range()];
                if key_text != "value" && key_text != "path" {
                    continue;
                }
                if let Some(text) = kotlin_string_from_value(value, source) {
                    return text;
                }
            }
        }
    }
    String::new()
}

/// `@RequestMapping(method = [RequestMethod.DELETE])`'s HTTP method — a
/// `navigation_expression` (Kotlin's `field_access` equivalent, no fields
/// of its own: `RequestMethod` then the identifier after the dot,
/// positionally), array-wrapped per Kotlin's own array-literal requirement.
/// `None` (defaulting to `"ANY"`) if there's no `method =` element.
fn kotlin_annotation_method_override(node: Node, source: &str) -> Option<String> {
    let inner = node.named_child(0)?;
    if inner.kind() != "constructor_invocation" {
        return None;
    }
    let value_arguments = child_by_kind(inner, "value_arguments")?;
    let mut cursor = value_arguments.walk();
    for arg in value_arguments.named_children(&mut cursor) {
        if arg.kind() != "value_argument" {
            continue;
        }
        let Some((Some(key_node), value)) = kotlin_value_argument_parts(arg) else {
            continue;
        };
        if &source[key_node.byte_range()] != "method" {
            continue;
        }
        let nav = match value.kind() {
            "navigation_expression" => Some(value),
            "collection_literal" => value.named_child(0).filter(|c| c.kind() == "navigation_expression"),
            _ => None,
        };
        if let Some(nav) = nav
            && let Some(field) = nav.named_child(1)
        {
            return Some(source[field.byte_range()].to_string());
        }
    }
    None
}

/// The class's own `@RequestMapping` base path, if its `modifiers` child
/// carries one — `@RestController`/`@Controller` alone contribute no path.
fn kotlin_class_base_path(class_node: Node, source: &str) -> String {
    let Some(modifiers) = child_by_kind(class_node, "modifiers") else {
        return String::new();
    };
    let mut cursor = modifiers.walk();
    for annotation in modifiers.named_children(&mut cursor) {
        if annotation.kind() != "annotation" {
            continue;
        }
        if kotlin_annotation_name(annotation, source).as_deref() == Some("RequestMapping") {
            return kotlin_annotation_path(annotation, source);
        }
    }
    String::new()
}

/// `(http_method, path)` from the first recognized mapping annotation on
/// `function_node`'s `modifiers` child, if any — everything else (the
/// recognized-annotation list, "method-level annotation is what makes it
/// an endpoint" rule, ignoring other annotations) is identical to the Java
/// side's `method_mapping`.
fn kotlin_function_mapping(function_node: Node, source: &str) -> Option<(String, String)> {
    let modifiers = child_by_kind(function_node, "modifiers")?;
    let mut cursor = modifiers.walk();
    for annotation in modifiers.named_children(&mut cursor) {
        if annotation.kind() != "annotation" {
            continue;
        }
        let Some(name) = kotlin_annotation_name(annotation, source) else {
            continue;
        };
        let Some(fixed) = fixed_http_method(&name) else {
            continue;
        };
        let http_method = match fixed {
            Some(method) => method.to_string(),
            None => kotlin_annotation_method_override(annotation, source).unwrap_or_else(|| "ANY".to_string()),
        };
        let path = kotlin_annotation_path(annotation, source);
        return Some((http_method, path));
    }
    None
}

/// Every Spring MVC endpoint declared in `tree` — Kotlin counterpart to
/// `java_endpoints_in_file`, walking `class_body`'s `function_declaration`
/// children (mirroring `kotlin_members.rs`'s own traversal) instead of
/// Java's `method_declaration` children.
fn kotlin_routes(tree: &Tree, source: &str) -> Vec<HttpRoute> {
    let mut out = Vec::new();
    collect_kotlin_routes(tree.root_node(), source, &mut out);
    out
}

fn collect_kotlin_routes(node: Node, source: &str, out: &mut Vec<HttpRoute>) {
    if node.kind() == "class_declaration"
        && let Some(name_node) = node.child_by_field_name("name")
        && let Some(body) = child_by_kind(node, "class_body")
    {
        let controller_name = source[name_node.byte_range()].to_string();
        let base_path = kotlin_class_base_path(node, source);

        let mut cursor = body.walk();
        for member in body.named_children(&mut cursor) {
            if member.kind() != "function_declaration" {
                continue;
            }
            let Some((http_method, method_path)) = kotlin_function_mapping(member, source) else {
                continue;
            };
            let Some(handler_name_node) = member.child_by_field_name("name") else {
                continue;
            };
            out.push(HttpRoute {
                method: http_method,
                path: join_paths(&base_path, &method_path),
                owner: controller_name.clone(),
                handler: source[handler_name_node.byte_range()].to_string(),
                handler_byte: handler_name_node.start_byte(),
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_kotlin_routes(child, source, out);
    }
}

#[cfg(test)]
#[path = "http_routes_test.rs"]
mod http_routes_test;
