use super::*;
use fg_extension::ProblemSeverity;

/// Captured verbatim from a real `checkstyle -c sun_checks.xml -f xml`
/// run against a small fixture file with several real violations —
/// grammar shape verified fresh, not assumed, per this project's own
/// discipline for external report formats.
const REAL_REPORT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<checkstyle version="8.36.1">
<file name="/tmp/fixture/Bad.java">
<error line="1" severity="error" message="Missing a package-info.java file." source="com.puppycrawl.tools.checkstyle.checks.javadoc.JavadocPackageCheck"/>
<error line="3" column="8" severity="error" message="Unused import - java.util.List." source="com.puppycrawl.tools.checkstyle.checks.imports.UnusedImportsCheck"/>
<error line="7" column="16" severity="warning" message="&apos;=&apos; is not preceded with whitespace." source="com.puppycrawl.tools.checkstyle.checks.whitespace.WhitespaceAroundCheck"/>
</file>
</checkstyle>
"#;

#[test]
fn parse_checkstyle_xml_extracts_every_error_with_its_file() {
    let findings = parse_checkstyle_xml(REAL_REPORT).expect("parses");
    assert_eq!(findings.len(), 3);
    assert_eq!(findings[0].file, PathBuf::from("/tmp/fixture/Bad.java"));
    assert_eq!(findings[0].line, 1);
    assert_eq!(findings[0].column, None);
    assert_eq!(findings[0].severity, ProblemSeverity::Error);
    assert_eq!(findings[1].column, Some(8));
}

#[test]
fn parse_checkstyle_xml_maps_non_error_severity_to_warning() {
    let findings = parse_checkstyle_xml(REAL_REPORT).expect("parses");
    assert_eq!(findings[2].severity, ProblemSeverity::Warning);
}

#[test]
fn parse_checkstyle_xml_unescapes_xml_entities_in_the_message() {
    let findings = parse_checkstyle_xml(REAL_REPORT).expect("parses");
    assert_eq!(findings[2].message, "'=' is not preceded with whitespace.");
}

#[test]
fn parse_checkstyle_xml_rejects_an_error_outside_any_file() {
    let xml = r#"<checkstyle version="8.36.1"><error line="1" severity="error" message="x"/></checkstyle>"#;
    assert!(parse_checkstyle_xml(xml).is_err());
}

#[test]
fn a_checkstyle_finding_becomes_a_point_finding() {
    let finding = into_finding(CheckstyleFinding {
        file: PathBuf::from("/p/Bad.java"),
        line: 2,
        column: Some(7),
        severity: ProblemSeverity::Warning,
        message: "example".to_string(),
    });
    assert_eq!(finding.file, PathBuf::from("/p/Bad.java"));
    assert_eq!((finding.line, finding.column, finding.end), (2, Some(7), None));
    assert_eq!(finding.severity, ProblemSeverity::Warning);
    assert_eq!(finding.message, "example");
}

