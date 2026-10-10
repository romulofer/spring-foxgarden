use super::*;

fn parse(language: tree_sitter::Language, source: &str) -> Tree {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).expect("a bundled grammar must load");
    parser.parse(source, None).expect("parse")
}

fn parsed(language_id: &str, source: &str) -> Tree {
    match language_id {
        JAVA => parse(tree_sitter_java::LANGUAGE.into(), source),
        _ => parse(tree_sitter_kotlin_ng::LANGUAGE.into(), source),
    }
}

/// The edit as the `(byte, text)` pair the assertions below read.
fn edit(tree: &Tree, source: &str, language_id: &str, qualified_name: &str) -> (usize, String) {
    let edit = import_edit(tree, source, language_id, qualified_name).expect("an edit");
    (edit.byte, edit.text)
}

#[test]
fn a_language_that_is_not_jvm_gets_no_edit() {
    let source = "class Foo {}\n";
    let tree = parsed(JAVA, source);
    assert!(import_edit(&tree, source, "yaml", "a.B").is_none());
}

#[test]
fn java_existing_imports_strips_the_terminator_and_reports_document_order() {
    let source = "package com.example;\n\nimport java.util.List;\nimport java.util.Map;\n\nclass Foo {}\n";
    let tree = parsed(JAVA, source);
    let imports = existing_imports(&tree, source, JAVA);
    assert_eq!(
        imports.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(),
        vec!["java.util.List", "java.util.Map"]
    );
    assert_eq!(&source[imports[0].byte_range.clone()], "import java.util.List;");
}

#[test]
fn java_static_import_has_the_static_keyword_stripped_from_its_own_path() {
    let source = "import static org.junit.Assert.assertEquals;\n\nclass Foo {}\n";
    let tree = parsed(JAVA, source);
    let imports = existing_imports(&tree, source, JAVA);
    assert_eq!(imports[0].path, "org.junit.Assert.assertEquals");
}

#[test]
fn kotlin_existing_imports_has_no_terminator_to_strip() {
    let source = "package com.example\n\nimport java.util.List\n\nclass Foo\n";
    let tree = parsed(KOTLIN, source);
    let imports = existing_imports(&tree, source, KOTLIN);
    assert_eq!(imports[0].path, "java.util.List");
}

#[test]
fn a_language_with_no_import_vocabulary_reports_none() {
    let source = "key: value\n";
    let tree = parsed(JAVA, source);
    assert!(existing_imports(&tree, source, "yaml").is_empty());
}

#[test]
fn import_insertion_finds_the_alphabetically_correct_before_slot() {
    let existing = vec![
        ExistingImport {
            path: "java.util.List".to_string(),
            byte_range: 0..10,
        },
        ExistingImport {
            path: "java.util.Set".to_string(),
            byte_range: 20..30,
        },
    ];
    assert_eq!(
        import_insertion(&existing, "java.util.Map"),
        ImportInsertion::Before(20)
    );
}

#[test]
fn import_insertion_falls_back_to_after_the_last_import_when_new_path_sorts_last() {
    let existing = vec![
        ExistingImport {
            path: "java.util.List".to_string(),
            byte_range: 0..10,
        },
        ExistingImport {
            path: "java.util.Map".to_string(),
            byte_range: 20..30,
        },
    ];
    assert_eq!(
        import_insertion(&existing, "org.springframework.stereotype.Component"),
        ImportInsertion::AfterLast(30)
    );
}

#[test]
fn import_insertion_reports_already_imported_for_an_exact_match() {
    let existing = vec![ExistingImport {
        path: "org.springframework.stereotype.Component".to_string(),
        byte_range: 0..10,
    }];
    assert_eq!(
        import_insertion(&existing, "org.springframework.stereotype.Component"),
        ImportInsertion::AlreadyImported
    );
}

#[test]
fn import_insertion_with_no_existing_imports_reports_after_last_zero() {
    assert_eq!(
        import_insertion(&[], "org.springframework.stereotype.Component"),
        ImportInsertion::AfterLast(0)
    );
}

/// A package that only starts with the letters of `static` keeps them.
#[test]
fn a_package_starting_with_static_is_not_mistaken_for_the_modifier() {
    let source = "import staticfiles.Config;\n\nclass Foo {}\n";
    let tree = parsed(JAVA, source);
    assert_eq!(existing_imports(&tree, source, JAVA)[0].path, "staticfiles.Config");

    let source = "import statistics.Mean\n\nclass Foo\n";
    let tree = parsed(KOTLIN, source);
    assert_eq!(existing_imports(&tree, source, KOTLIN)[0].path, "statistics.Mean");
}

#[test]
fn already_imported_yields_no_edit() {
    let source = "import org.springframework.stereotype.Component;\n\nclass Foo {}\n";
    let tree = parsed(JAVA, source);
    assert!(import_edit(&tree, source, JAVA, "org.springframework.stereotype.Component").is_none());
}

#[test]
fn java_with_no_imports_at_all_inserts_after_the_package_declaration() {
    let source = "package com.example;\n\nclass Foo {}\n";
    let tree = parsed(JAVA, source);
    let (byte, insertion) = edit(&tree, source, JAVA, "org.springframework.stereotype.Component");
    assert_eq!(byte, "package com.example;".len());
    assert_eq!(insertion, "\n\nimport org.springframework.stereotype.Component;");

    let mut result = source.to_string();
    result.insert_str(byte, &insertion);
    assert_eq!(
        result,
        "package com.example;\n\nimport org.springframework.stereotype.Component;\n\nclass Foo {}\n"
    );
}

#[test]
fn java_with_no_package_and_no_imports_inserts_at_the_very_start() {
    let source = "class Foo {}\n";
    let tree = parsed(JAVA, source);
    let (byte, insertion) = edit(&tree, source, JAVA, "org.springframework.stereotype.Component");
    assert_eq!(byte, 0);
    assert_eq!(insertion, "import org.springframework.stereotype.Component;\n\n");
}

#[test]
fn java_inserts_at_the_correct_alphabetical_slot_among_existing_imports() {
    // "org.springframework.stereotype.Component" sorts after
    // "java.util.List" (`j` < `o`) but before "org.springframework.
    // web.bind.annotation.RestController" (`stereotype` < `web`) —
    // genuinely bracketing the insertion point, unlike two plain
    // `java.util.*` imports (which both sort *before* any
    // `org.springframework.*` one).
    let source =
        "import java.util.List;\nimport org.springframework.web.bind.annotation.RestController;\n\nclass Foo {}\n";
    let tree = parsed(JAVA, source);
    let (byte, insertion) = edit(&tree, source, JAVA, "org.springframework.stereotype.Component");

    let mut result = source.to_string();
    result.insert_str(byte, &insertion);
    assert_eq!(
        result,
        "import java.util.List;\nimport org.springframework.stereotype.Component;\nimport org.springframework.web.bind.annotation.RestController;\n\nclass Foo {}\n"
    );
}

#[test]
fn java_appends_after_the_last_import_when_the_new_one_sorts_last() {
    let source = "import java.util.List;\nimport java.util.Map;\n\nclass Foo {}\n";
    let tree = parsed(JAVA, source);
    let (byte, insertion) = edit(&tree, source, JAVA, "org.springframework.beans.factory.annotation.Value");

    let mut result = source.to_string();
    result.insert_str(byte, &insertion);
    assert_eq!(
        result,
        "import java.util.List;\nimport java.util.Map;\nimport org.springframework.beans.factory.annotation.Value;\n\nclass Foo {}\n"
    );
}

#[test]
fn kotlin_import_has_no_semicolon() {
    let source = "package com.example\n\nclass Foo\n";
    let tree = parsed(KOTLIN, source);
    let (_byte, insertion) = edit(&tree, source, KOTLIN, "org.springframework.stereotype.Component");
    assert_eq!(insertion, "\n\nimport org.springframework.stereotype.Component");
}
