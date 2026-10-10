//! Making a name resolve in a Java or Kotlin file: which `import`s the file
//! already has, and where a new one belongs, alphabetically.
//!
//! Spring-annotation auto-import is the consumer: accepting `@Component`
//! has to add `import org.springframework.stereotype.Component` unless it is
//! already there. The editor splices whatever edit `import_edit` returns and
//! knows nothing about statements, terminators or `package` lines.

use std::ops::Range;

use fg_extension::ImportEdit;
use tree_sitter::Tree;

use crate::{JAVA, KOTLIN};

/// The node kind of one import statement in `language_id`'s grammar. Both
/// are always direct children of the root node, never nested.
fn import_kind(language_id: &str) -> Option<&'static str> {
    match language_id {
        JAVA => Some("import_declaration"),
        KOTLIN => Some("import"),
        _ => None,
    }
}

/// One `import` statement already in the file, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingImport {
    /// The imported path, with any Java `static` prefix and the language's
    /// own statement terminator (`;` for Java, none for Kotlin) stripped —
    /// e.g. `"org.springframework.stereotype.Component"`, not `"import
    /// org.springframework.stereotype.Component;"`. An aliased Kotlin import
    /// (`import x.Y as Z`) is kept as-is, `"x.Y as Z"` — rare enough for
    /// Spring annotations that it is not special-cased.
    path: String,
    /// The whole statement's byte range (including Java's trailing `;`, not
    /// including a trailing newline).
    byte_range: Range<usize>,
}

/// Every `import` statement directly in `tree`'s root, in document order.
/// Empty for a language with no import vocabulary or a file with none.
fn existing_imports(tree: &Tree, source: &str, language_id: &str) -> Vec<ExistingImport> {
    let Some(kind) = import_kind(language_id) else {
        return Vec::new();
    };

    let mut cursor = tree.root_node().walk();
    tree.root_node()
        .children(&mut cursor)
        .filter(|child| child.kind() == kind)
        .map(|child| ExistingImport {
            path: import_path_text(&source[child.start_byte()..child.end_byte()]).to_string(),
            byte_range: child.byte_range(),
        })
        .collect()
}

/// Strips `import`/`import static`'s own keyword(s) and any trailing
/// `;`/whitespace from a raw statement's text, leaving just the dotted path.
fn import_path_text(text: &str) -> &str {
    let text = strip_keyword(text, "import");
    let text = strip_keyword(text, "static");
    text.trim_end_matches(';').trim()
}

/// `text` without a leading `keyword`, only when it is the whole word — a
/// package that merely starts with the same letters (`staticfiles`,
/// `statistics`) is part of the path, not a modifier.
fn strip_keyword<'a>(text: &'a str, keyword: &str) -> &'a str {
    match text.strip_prefix(keyword) {
        Some(rest) if rest.starts_with(char::is_whitespace) => rest.trim_start(),
        _ => text,
    }
}

/// Where a new `path` belongs among `existing` (assumed in document order,
/// not necessarily sorted — a caller inserting one import into an otherwise
/// unsorted block still lands it in the *locally* correct alphabetical slot,
/// without re-sorting the file's existing imports, which isn't its job).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportInsertion {
    /// `path` is already imported exactly — nothing to insert.
    AlreadyImported,
    /// Insert immediately before this existing import's start byte — the
    /// first one (in document order) whose own path sorts after `path`.
    Before(usize),
    /// No existing import sorts after `path` — insert immediately after this
    /// last existing import's end byte. `0` when there are no imports at
    /// all; the caller decides the real anchor.
    AfterLast(usize),
}

/// Pure string comparison — no package-relative shortening or
/// wildcard-import awareness (an existing `import org.springframework.
/// stereotype.*;` covering `path` isn't detected as "already imported," a
/// known, disclosed simplification).
fn import_insertion(existing: &[ExistingImport], path: &str) -> ImportInsertion {
    if existing.iter().any(|e| e.path == path) {
        return ImportInsertion::AlreadyImported;
    }
    match existing.iter().find(|e| e.path.as_str() > path) {
        Some(next) => ImportInsertion::Before(next.byte_range.start),
        None => ImportInsertion::AfterLast(existing.last().map_or(0, |last| last.byte_range.end)),
    }
}

/// The edit that imports `qualified_name` into `source`, or `None` when it
/// already is, or `language_id` is not a JVM language.
pub fn import_edit(tree: &Tree, source: &str, language_id: &str, qualified_name: &str) -> Option<ImportEdit> {
    import_kind(language_id)?;
    let existing = existing_imports(tree, source, language_id);

    let statement = match language_id {
        JAVA => format!("import {qualified_name};"),
        _ => format!("import {qualified_name}"),
    };

    let (byte, text) = match import_insertion(&existing, qualified_name) {
        ImportInsertion::AlreadyImported => return None,
        ImportInsertion::Before(byte) => (byte, format!("{statement}\n")),
        ImportInsertion::AfterLast(byte) if !existing.is_empty() => (byte, format!("\n{statement}")),
        ImportInsertion::AfterLast(_) => first_import_insertion_point(tree, &statement),
    };
    Some(ImportEdit { byte, text })
}

/// Where to put the file's very first import: right after the `package`
/// declaration (Java `package_declaration`, Kotlin `package_header` — both,
/// like an import itself, always a direct child of the root node), or at the
/// very start of the file if there's no package declaration either (an
/// unusual but valid default-package Java file).
fn first_import_insertion_point(tree: &Tree, statement: &str) -> (usize, String) {
    let mut cursor = tree.root_node().walk();
    let package_end = tree
        .root_node()
        .children(&mut cursor)
        .find(|child| matches!(child.kind(), "package_declaration" | "package_header"))
        .map(|node| node.end_byte());

    match package_end {
        Some(byte) => (byte, format!("\n\n{statement}")),
        None => (0, format!("{statement}\n\n")),
    }
}

#[cfg(test)]
#[path = "imports_test.rs"]
mod imports_test;
