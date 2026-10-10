use fg_extension::{Extension, GrammarSource, Registry};

use super::{SpringExtension, JAVA, JDK, KOTLIN};

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
    // The extension contributes the JVM languages and nothing else — a YAML
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

#[test]
fn contributes_the_jdk_runtime_kind() {
    let runtimes = registry().runtimes();
    assert_eq!(runtimes.len(), 1);
    assert_eq!(runtimes[0].id(), JDK);
    assert_eq!(runtimes[0].display_name(), "JDKs");
    assert_eq!(runtimes[0].contribution.item_name, "JDK");
}

#[test]
fn a_directory_without_a_java_binary_is_not_a_jdk() {
    let dir = tempfile::tempdir().unwrap();
    let error = registry().runtimes()[0].detect(dir.path()).unwrap_err();
    assert!(error.contains("couldn't run"), "{error}");
}

#[test]
fn another_runtime_kinds_ids_are_not_answered() {
    let dir = tempfile::tempdir().unwrap();
    assert!(SpringExtension.detect_runtime("node", dir.path()).is_err());
    assert!(SpringExtension.discover_runtimes("node").is_empty());
}

#[cfg(unix)]
mod fake_jdk {
    use std::path::{Path, PathBuf};

    use super::registry;

    fn fake_jdk(dir: &Path, banner: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let bin = dir.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let script = bin.join("java");
        std::fs::write(&script, format!("#!/bin/sh\necho '{banner}' >&2\n")).unwrap();
        let mut perms = std::fs::metadata(&script).unwrap().permissions();
        perms.set_mode(perms.mode() | 0o755);
        std::fs::set_permissions(&script, perms).unwrap();
        wait_until_executable(&script);
        dir.to_path_buf()
    }

    /// Another test thread that forks while this one is writing `script`
    /// carries the write handle into its child until that child execs, and
    /// running the script meanwhile fails with "Text file busy" (ETXTBSY) —
    /// an occasional failure under parallel `cargo test` that has nothing to
    /// do with what the test checks. Runs it until that window has passed.
    fn wait_until_executable(script: &Path) {
        for _ in 0..100 {
            match std::process::Command::new(script).output() {
                Err(err) if err.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                _ => return,
            }
        }
    }

    #[test]
    fn detect_records_the_real_detected_version() {
        let dir = tempfile::tempdir().unwrap();
        let home = fake_jdk(dir.path(), "openjdk version \"17.0.4\" 2022-07-19 LTS");

        let install = registry().runtimes()[0].detect(&home).expect("detects a fake JDK 17");

        assert_eq!(install.major_version, 17);
        assert_eq!(install.home, home);
        assert_eq!(install.label, "Java 17");
    }

    #[test]
    fn detect_reads_the_legacy_one_dot_eight_banner() {
        let dir = tempfile::tempdir().unwrap();
        let home = fake_jdk(dir.path(), "java version \"1.8.0_292\"");

        assert_eq!(registry().runtimes()[0].detect(&home).unwrap().major_version, 8);
    }
}
