use super::*;

#[test]
fn candidates_carry_their_own_import_path() {
    let candidates = candidates(JAVA);
    let component = candidates.iter().find(|c| c.name == "Component").unwrap();
    assert_eq!(component.qualified_name, "org.springframework.stereotype.Component");
}

#[test]
fn kotlin_offers_the_same_annotations() {
    assert_eq!(candidates(KOTLIN), candidates(JAVA));
}

#[test]
fn other_languages_get_none() {
    assert!(candidates("yaml").is_empty());
}
