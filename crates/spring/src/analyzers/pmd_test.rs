use super::*;
use fg_extension::ProblemSeverity;

/// Captured verbatim from a real
/// `pmd check -R rulesets/java/quickstart.xml -f xml --no-cache` run
/// (PMD 7.26.0) against a small fixture file with a real
/// `CompareObjectsWithEquals`/`UseEqualsToCompareStrings`/
/// `UnusedLocalVariable` violation each.
const REAL_PMD_REPORT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<pmd xmlns="http://pmd.sourceforge.net/report/2.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xsi:schemaLocation="http://pmd.sourceforge.net/report/2.0.0 https://pmd.github.io/schema/report_2_0_0.xsd" version="7.26.0" timestamp="2026-07-28T07:04:02.976">
<file name="/tmp/fixture/Bad2.java">
<violation beginline="5" endline="5" begincolumn="13" endcolumn="19" rule="CompareObjectsWithEquals" ruleset="Error Prone" package="demo" class="Bad2" method="check" externalInfoUrl="https://docs.pmd-code.org/snapshot/pmd_rules_java_errorprone.html#compareobjectswithequals" priority="3">
Use equals() to compare object references.
</violation>
<violation beginline="5" endline="5" begincolumn="13" endcolumn="19" rule="UseEqualsToCompareStrings" ruleset="Error Prone" package="demo" class="Bad2" method="check" externalInfoUrl="https://docs.pmd-code.org/snapshot/pmd_rules_java_errorprone.html#useequalstocomparestrings" priority="3">
Use equals() to compare strings instead of '==' or '!='
</violation>
<violation beginline="8" endline="8" begincolumn="16" endcolumn="17" rule="UnusedLocalVariable" ruleset="Best Practices" package="demo" class="Bad2" method="check" variable="s" externalInfoUrl="https://docs.pmd-code.org/snapshot/pmd_rules_java_bestpractices.html#unusedlocalvariable" priority="3">
Avoid unused local variables such as 's'.
</violation>
</file>
</pmd>
"#;

#[test]
fn parse_pmd_xml_extracts_every_violation_with_its_file_and_range() {
    let findings = parse_pmd_xml(REAL_PMD_REPORT).expect("parses");
    assert_eq!(findings.len(), 3);
    assert_eq!(findings[0].file, PathBuf::from("/tmp/fixture/Bad2.java"));
    assert_eq!(findings[0].begin_line, 5);
    assert_eq!(findings[0].begin_column, 13);
    assert_eq!(findings[0].end_line, 5);
    assert_eq!(findings[0].end_column, 19);
    assert_eq!(findings[0].priority, 3);
}

#[test]
fn parse_pmd_xml_trims_the_message_from_the_element_text_content() {
    let findings = parse_pmd_xml(REAL_PMD_REPORT).expect("parses");
    assert_eq!(findings[0].message, "Use equals() to compare object references.");
    assert_eq!(findings[2].message, "Avoid unused local variables such as 's'.");
}

#[test]
fn parse_pmd_xml_rejects_a_violation_outside_any_file() {
    let xml = r#"<pmd version="7.26.0"><violation beginline="1" endline="1" begincolumn="1" endcolumn="1" priority="3">x</violation></pmd>"#;
    assert!(parse_pmd_xml(xml).is_err());
}

#[test]
fn parse_pmd_xml_with_no_violations_returns_an_empty_list() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<pmd xmlns="http://pmd.sourceforge.net/report/2.0.0" version="7.26.0" timestamp="2026-07-28T07:00:03.223">
</pmd>
"#;
    assert_eq!(parse_pmd_xml(xml).expect("parses"), vec![]);
}

#[test]
fn pmd_severity_maps_high_and_medium_high_to_error_and_the_rest_to_warning() {
    assert_eq!(pmd_severity(1), ProblemSeverity::Error);
    assert_eq!(pmd_severity(2), ProblemSeverity::Error);
    assert_eq!(pmd_severity(3), ProblemSeverity::Warning);
    assert_eq!(pmd_severity(4), ProblemSeverity::Warning);
    assert_eq!(pmd_severity(5), ProblemSeverity::Warning);
}

#[test]
fn a_pmd_violation_keeps_its_inclusive_begin_and_end() {
    let finding = into_finding(PmdFinding {
        file: PathBuf::from("/p/Bad2.java"),
        begin_line: 5,
        begin_column: 13,
        end_line: 5,
        end_column: 19,
        priority: 3,
        message: "example".to_string(),
    });
    assert_eq!((finding.line, finding.column, finding.end), (5, Some(13), Some((5, 19))));
    assert_eq!(finding.severity, ProblemSeverity::Warning);
}

