use super::*;

fn contribution(id: &str) -> AnalyzerContribution {
    contributions().into_iter().find(|a| a.id == id).expect("contributed")
}

#[test]
fn the_three_tools_are_contributed_in_the_order_the_settings_dialog_lists_them() {
    let ids: Vec<_> = contributions().into_iter().map(|a| a.id).collect();
    assert_eq!(ids, [CHECKSTYLE, PMD, SPOTBUGS]);
}

#[test]
fn only_spotbugs_needs_compiled_classes_and_has_no_config_field() {
    assert_eq!(contribution(CHECKSTYLE).input, AnalyzerInput::Sources);
    assert_eq!(contribution(PMD).input, AnalyzerInput::Sources);
    assert_eq!(contribution(SPOTBUGS).input, AnalyzerInput::BuildOutput);
    assert!(contribution(CHECKSTYLE).config_label.is_some());
    assert!(contribution(PMD).config_label.is_some());
    assert!(contribution(SPOTBUGS).config_label.is_none());
}

#[test]
fn download_urls_match_the_real_asset_layout_verified_against_each_release() {
    let url = |id| contribution(id).install.unwrap().download_url;
    assert_eq!(
        url(CHECKSTYLE),
        "https://github.com/checkstyle/checkstyle/releases/download/checkstyle-10.26.1/checkstyle-10.26.1-all.jar"
    );
    assert_eq!(
        url(PMD),
        "https://github.com/pmd/pmd/releases/download/pmd_releases/7.26.0/pmd-dist-7.26.0-bin.zip"
    );
    assert_eq!(
        url(SPOTBUGS),
        "https://github.com/spotbugs/spotbugs/releases/download/4.10.3/spotbugs-4.10.3.zip"
    );
}

#[test]
fn every_install_is_pinned_to_a_sha256() {
    for analyzer in contributions() {
        let install = analyzer.install.unwrap();
        assert_eq!(install.sha256.len(), 64, "{}", analyzer.id);
        assert!(install.sha256.chars().all(|c| c.is_ascii_hexdigit()), "{}", analyzer.id);
    }
}

#[test]
fn latest_release_tags_reduce_to_comparable_versions() {
    let release = |id| contribution(id).install.unwrap().latest_release.unwrap();
    assert_eq!(release(PMD).api_url, "https://api.github.com/repos/pmd/pmd/releases/latest");
    assert_eq!(release(CHECKSTYLE).tag_prefix, "checkstyle-");
    assert_eq!(release(PMD).tag_prefix, "pmd_releases/");
    assert_eq!(release(SPOTBUGS).tag_prefix, "");
}

#[test]
fn an_unknown_analyzer_is_an_error() {
    let run = AnalyzerRun {
        binary: "tool".into(),
        config: String::new(),
        project_root: "/p".into(),
        classes_dir: None,
    };
    assert!(super::run("nope", &run).is_err());
}
