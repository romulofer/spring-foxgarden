//! Java's and Kotlin's live templates and reserved words — what typing `sout`
//! then Tab expands to, and which words completion offers as keywords.

use fg_extension::Snippet;

use crate::{JAVA, KOTLIN};

/// Built-in Java live templates. The editor lets a user's own templates
/// (Help > Live Templates…) take priority when a trigger collides.
const JAVA_SNIPPETS: &[(&str, &str)] = &[
    ("sout", "System.out.println(${cursor});"),
    ("souf", "System.out.printf(${cursor});"),
    ("serr", "System.err.println(${cursor});"),
    ("psvm", "public static void main(String[] args) {\n    ${cursor}\n}"),
    ("fori", "for (int i = 0; i < ${cursor}; i++) {\n    \n}"),
    ("iter", "for (var item : ${cursor}) {\n    \n}"),
    ("ifn", "if (${cursor} == null) {\n    \n}"),
    ("inn", "if (${cursor} != null) {\n    \n}"),
    ("trycatch", "try {\n    ${cursor}\n} catch (Exception e) {\n    e.printStackTrace();\n}"),
];

/// Built-in Kotlin live templates.
const KOTLIN_SNIPPETS: &[(&str, &str)] = &[
    ("sout", "println(${cursor})"),
    ("serr", "System.err.println(${cursor})"),
    ("main", "fun main() {\n    ${cursor}\n}"),
    ("fori", "for (i in 0 until ${cursor}) {\n    \n}"),
    ("ifn", "if (${cursor} == null) {\n    \n}"),
    ("inn", "if (${cursor} != null) {\n    \n}"),
    ("trycatch", "try {\n    ${cursor}\n} catch (e: Exception) {\n    e.printStackTrace()\n}"),
];

/// Java's reserved words (`CompletionKind::Keyword` candidates, `SPEC.md`
/// §1's "Also includes") — not exhaustive of every contextual/restricted
/// identifier the JLS defines (`sealed`, `permits`, `yield` etc. are
/// context-sensitive, not reserved everywhere), just the always-reserved
/// set a word-completion popup is worth offering.
const JAVA_KEYWORDS: &[&str] = &[
    "abstract",
    "assert",
    "boolean",
    "break",
    "byte",
    "case",
    "catch",
    "char",
    "class",
    "const",
    "continue",
    "default",
    "do",
    "double",
    "else",
    "enum",
    "extends",
    "final",
    "finally",
    "float",
    "for",
    "goto",
    "if",
    "implements",
    "import",
    "instanceof",
    "int",
    "interface",
    "long",
    "native",
    "new",
    "package",
    "private",
    "protected",
    "public",
    "return",
    "short",
    "static",
    "strictfp",
    "super",
    "switch",
    "synchronized",
    "this",
    "throw",
    "throws",
    "transient",
    "try",
    "void",
    "volatile",
    "while",
    "true",
    "false",
    "null",
    "var",
];

/// Kotlin's hard keywords — always reserved, unlike its soft/modifier
/// keywords (`data`, `sealed`, `internal`, etc.), which are valid
/// identifiers elsewhere and so are a worse fit for an unconditional
/// candidate list.
const KOTLIN_KEYWORDS: &[&str] = &[
    "as",
    "break",
    "class",
    "continue",
    "do",
    "else",
    "false",
    "for",
    "fun",
    "if",
    "in",
    "interface",
    "is",
    "null",
    "object",
    "package",
    "return",
    "super",
    "this",
    "throw",
    "true",
    "try",
    "typealias",
    "typeof",
    "val",
    "var",
    "when",
    "while",
];

fn snippets_of(table: &[(&str, &str)]) -> Vec<Snippet> {
    table.iter().map(|(trigger, body)| Snippet::new(trigger, body)).collect()
}

pub fn snippets(language_id: &str) -> Vec<Snippet> {
    match language_id {
        JAVA => snippets_of(JAVA_SNIPPETS),
        KOTLIN => snippets_of(KOTLIN_SNIPPETS),
        _ => Vec::new(),
    }
}

pub fn keywords(language_id: &str) -> Vec<String> {
    let table = match language_id {
        JAVA => JAVA_KEYWORDS,
        KOTLIN => KOTLIN_KEYWORDS,
        _ => return Vec::new(),
    };
    table.iter().map(|word| word.to_string()).collect()
}

#[cfg(test)]
#[path = "snippets_test.rs"]
mod snippets_test;
