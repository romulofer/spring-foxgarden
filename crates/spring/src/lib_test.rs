use fg_extension::{Extension, GrammarSource, Registry};

use super::{SpringExtension, JAVA, KOTLIN};

fn registry() -> Registry {
    let mut registry = Registry::new();
    registry.register(Box::new(SpringExtension)).expect("spring must register");
    registry
}

#[test]
fn registers_against_the_editors_own_registry() {
    let registry = registry();
    assert_eq!(registry.extensions().len(), 1);
    assert_eq!(registry.extensions()[0].id, "spring");
}

#[test]
fn recognizes_java_and_kotlin_source_files() {
    let registry = registry();
    let java = registry
        .language_for_path(std::path::Path::new("src/main/java/com/example/Main.java"))
        .expect("a .java file is Java");
    assert_eq!(java.language.id, JAVA);
    assert_eq!(java.language.display_name, "Java");

    let kotlin = registry
        .language_for_path(std::path::Path::new("Main.kt"))
        .expect("a .kt file is Kotlin");
    assert_eq!(kotlin.language.id, KOTLIN);
}

#[test]
fn claims_no_other_file_type() {
    let registry = registry();
    // The add-on contributes the JVM languages and nothing else — a YAML
    // file in a Spring project is still the editor's own `file-types`
    // extension's business, not this one's. Asserted because the tempting
    // shortcut (claiming `.yml` here, since Spring reads
    // `application.yml`) would make this extension undroppable.
    for path in ["application.yml", "pom.xml", "Dockerfile", "notes.txt"] {
        assert!(
            registry.language_for_path(std::path::Path::new(path)).is_none(),
            "{path} must not be claimed by the spring extension"
        );
    }
}

/// Both grammars are real parsers, not just declarations: each one parses a
/// snippet of its own language without errors, with its highlight query
/// compiling against it. A query is written against one specific grammar's
/// node names, so the pairing is what can silently rot when either side is
/// bumped — nothing else in either repository checks it.
#[test]
fn every_contributed_grammar_parses_its_language_and_compiles_its_query() {
    let sources = [
        (JAVA, "class Main { void run() { int x = 1; } }"),
        // Kotlin's grammar treats a newline as a statement terminator, so
        // this snippet is written the way real Kotlin is rather than
        // squeezed onto one line like the Java one above.
        (KOTLIN, "class Main {\n    fun run() {\n        val x = 1\n    }\n}\n"),
    ];
    for (language_id, source) in sources {
        let contributions = SpringExtension.contributions();
        let grammar = contributions
            .grammars
            .iter()
            .find(|g| g.language_id == language_id)
            .unwrap_or_else(|| panic!("{language_id} must contribute a grammar"));
        let GrammarSource::Builtin(language_fn) = grammar.source else {
            panic!("{language_id}'s grammar is compiled in, not loaded at runtime");
        };
        let ts: tree_sitter::Language = language_fn.into();

        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&ts)
            .unwrap_or_else(|e| panic!("{language_id}'s grammar must load: {e}"));
        let tree = parser.parse(source, None).expect("parse must produce a tree");
        assert!(
            !tree.root_node().has_error(),
            "{language_id} failed to parse its own snippet cleanly"
        );

        let query_source = grammar
            .highlight_query
            .as_ref()
            .unwrap_or_else(|| panic!("{language_id} must contribute a highlight query"));
        tree_sitter::Query::new(&ts, query_source)
            .unwrap_or_else(|e| panic!("{language_id}'s highlight query must compile: {e}"));
    }
}
