use super::*;

fn trigger<'a>(snippets: &'a [Snippet], name: &str) -> Option<&'a str> {
    snippets.iter().find(|s| s.trigger == name).map(|s| s.body.as_str())
}

#[test]
fn the_same_trigger_expands_differently_per_language() {
    assert_eq!(trigger(&snippets(JAVA), "sout"), Some("System.out.println(${cursor});"));
    assert_eq!(trigger(&snippets(KOTLIN), "sout"), Some("println(${cursor})"));
}

#[test]
fn java_has_psvm_and_kotlin_does_not() {
    assert!(trigger(&snippets(JAVA), "psvm").is_some());
    assert!(trigger(&snippets(KOTLIN), "psvm").is_none());
}

#[test]
fn another_language_has_no_snippets_or_keywords() {
    assert!(snippets("yaml").is_empty());
    assert!(keywords("yaml").is_empty());
}

#[test]
fn each_language_reserves_its_own_words() {
    assert!(keywords(JAVA).contains(&"synchronized".to_string()));
    assert!(!keywords(KOTLIN).contains(&"synchronized".to_string()));
    assert!(keywords(KOTLIN).contains(&"when".to_string()));
}
