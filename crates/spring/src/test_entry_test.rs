use super::*;

fn parse(language: tree_sitter::Language, source: &str) -> Tree {
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&language).expect("a bundled grammar must load");
    parser.parse(source, None).expect("parse")
}

fn java(source: &str) -> Vec<RunTarget> {
    test_targets(&parse(tree_sitter_java::LANGUAGE.into(), source), source, JAVA)
}

fn kotlin(source: &str) -> Vec<RunTarget> {
    test_targets(&parse(tree_sitter_kotlin_ng::LANGUAGE.into(), source), source, KOTLIN)
}

fn summary(targets: &[RunTarget]) -> Vec<(usize, &str)> {
    targets.iter().map(|t| (t.line, t.entry_point.as_str())).collect()
}

#[test]
fn a_java_test_class_and_each_test_method_get_a_marker() {
    let targets = java(
        "package com.example;\n\
         \n\
         class CalcTest {\n\
         \x20   @Test\n\
         \x20   void adds() {}\n\
         \n\
         \x20   void helper() {}\n\
         \n\
         \x20   @org.junit.jupiter.api.Test\n\
         \x20   public void subtracts() {}\n\
         }\n",
    );
    assert_eq!(
        summary(&targets),
        [
            (2, "com.example.CalcTest"),
            (4, "com.example.CalcTest#adds"),
            (9, "com.example.CalcTest#subtracts")
        ]
    );
    assert_eq!(targets[0].kind, RunKind::Test);
    assert!(targets[1..].iter().all(|t| t.kind == RunKind::SingleTest));
    assert_eq!(targets[0].label, "CalcTest");
    assert_eq!(targets[1].label, "adds");
}

#[test]
fn the_other_junit_test_annotations_count_and_unrelated_ones_do_not() {
    let targets = java(
        "class T {\n\
         \x20   @ParameterizedTest(name = \"x\")\n\
         \x20   void a() {}\n\
         \x20   @RepeatedTest(3)\n\
         \x20   void b() {}\n\
         \x20   @Deprecated\n\
         \x20   void c() {}\n\
         \x20   @Testable\n\
         \x20   void d() {}\n\
         }\n",
    );
    assert_eq!(summary(&targets), [(0, "T"), (2, "T#a"), (4, "T#b")]);
}

#[test]
fn a_nested_test_class_is_named_with_a_dollar_and_gets_its_own_marker() {
    let targets = java(
        "class Outer {\n\
         \x20   @Test void top() {}\n\
         \x20   @Nested class Inner {\n\
         \x20       @Test void deep() {}\n\
         \x20   }\n\
         }\n",
    );
    assert_eq!(
        summary(&targets),
        [(0, "Outer"), (1, "Outer#top"), (2, "Outer$Inner"), (3, "Outer$Inner#deep")]
    );
}

#[test]
fn a_file_without_tests_has_no_markers() {
    assert!(java("class Main { public static void main(String[] a) {} }").is_empty());
    assert!(kotlin("fun main() {}").is_empty());
}

#[test]
fn a_kotlin_test_class_and_its_methods_get_markers() {
    let targets = kotlin(
        "package com.example\n\
         \n\
         class CalcTest {\n\
         \x20   @Test\n\
         \x20   fun adds() {}\n\
         \n\
         \x20   fun helper() {}\n\
         \n\
         \x20   @Test fun `subtracts two`() {}\n\
         }\n",
    );
    assert_eq!(
        summary(&targets),
        [
            (2, "com.example.CalcTest"),
            (4, "com.example.CalcTest#adds"),
            (8, "com.example.CalcTest#subtracts two")
        ]
    );
}

#[test]
fn another_language_has_no_test_markers() {
    let source = "class A { @Test void a() {} }";
    let tree = parse(tree_sitter_java::LANGUAGE.into(), source);
    assert!(test_targets(&tree, source, "yaml").is_empty());
}
