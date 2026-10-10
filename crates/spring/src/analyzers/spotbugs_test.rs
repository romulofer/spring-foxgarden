use super::*;
use fg_extension::ProblemSeverity;

/// Captured verbatim from a real
/// `fb analyze -xml:withMessages -output report.xml <classes_dir>` run
/// (SpotBugs 4.10.3) against a small fixture class compiled with
/// `javac`, with a real `ES_COMPARING_PARAMETER_STRING_WITH_EQ` and a
/// real `OBL_UNSATISFIED_OBLIGATION` violation. The latter is the
/// specific case that disproves this codebase's own earlier, unverified
/// guess (`PLAN.md` Track 5's "Deferred" note) that the useful
/// `<SourceLine>` is the *last* direct child of `<BugInstance>` — here
/// it's the *first* of three direct-child `<SourceLine>`s, distinguished
/// only by its own `primary="true"` attribute. Trimmed of a third,
/// redundant `OS_OPEN_STREAM` violation and the `<BugPattern>`/
/// `<BugCode>`/`<FindBugsProfile>` tail (real but not read by this
/// parser), same "trim what the parser doesn't touch, keep what it
/// does" discipline `maven.rs`'s own `SIMPLE_POM` fixture already uses.
const REAL_SPOTBUGS_REPORT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<BugCollection version="4.10.3" sequence="0" timestamp="1787176559473" analysisTimestamp="1787176564882" release="">
  <Project projectName="">
    <Jar>/tmp/fixture/classes</Jar>
  </Project>
  <BugInstance type="ES_COMPARING_PARAMETER_STRING_WITH_EQ" priority="1" rank="14" abbrev="ES" category="BAD_PRACTICE" instanceHash="2718e3b7372e8143aea58a129489a108" instanceOccurrenceNum="0" instanceOccurrenceMax="0" cweid="595">
    <ShortMessage>Comparison of String parameter using == or !=</ShortMessage>
    <LongMessage>Comparison of String parameter using == or != in demo.Bad.compareStrings(String, String)</LongMessage>
    <Class classname="demo.Bad" primary="true">
      <SourceLine classname="demo.Bad" start="6" end="13" sourcefile="Bad.java" sourcepath="demo/Bad.java">
        <Message>At Bad.java:[lines 6-13]</Message>
      </SourceLine>
      <Message>In class demo.Bad</Message>
    </Class>
    <Method classname="demo.Bad" name="compareStrings" signature="(Ljava/lang/String;Ljava/lang/String;)Z" isStatic="true" primary="true">
      <SourceLine classname="demo.Bad" start="13" end="13" startBytecode="0" endBytecode="6" sourcefile="Bad.java" sourcepath="demo/Bad.java"/>
      <Message>In method demo.Bad.compareStrings(String, String)</Message>
    </Method>
    <Type descriptor="Ljava/lang/String;" role="TYPE_FOUND">
      <SourceLine classname="java.lang.String" start="140" end="4655" sourcefile="String.java" sourcepath="java/lang/String.java">
        <Message>At String.java:[lines 140-4655]</Message>
      </SourceLine>
      <Message>Actual type String</Message>
    </Type>
    <LocalVariable name="?" register="1" pc="1" role="LOCAL_VARIABLE_VALUE_OF">
      <Message>Value loaded from ?</Message>
    </LocalVariable>
    <SourceLine classname="demo.Bad" primary="true" start="13" end="13" startBytecode="2" endBytecode="2" sourcefile="Bad.java" sourcepath="demo/Bad.java">
      <Message>At Bad.java:[line 13]</Message>
    </SourceLine>
    <Property name="edu.umd.cs.findbugs.detect.RefComparisonWarningProperty.STATIC_AND_PARAMETER_IN_PUBLIC_METHOD" value="true"/>
  </BugInstance>
  <BugInstance type="OBL_UNSATISFIED_OBLIGATION" priority="2" rank="20" abbrev="OBL" category="EXPERIMENTAL" instanceHash="3c5b83a65d2ead90689f8a346a094d35" instanceOccurrenceNum="0" instanceOccurrenceMax="0">
    <ShortMessage>Method may fail to clean up stream or resource</ShortMessage>
    <LongMessage>demo.Bad.unclosedStream(String) may fail to clean up java.io.InputStream</LongMessage>
    <Class classname="demo.Bad" primary="true">
      <SourceLine classname="demo.Bad" start="6" end="13" sourcefile="Bad.java" sourcepath="demo/Bad.java">
        <Message>At Bad.java:[lines 6-13]</Message>
      </SourceLine>
      <Message>In class demo.Bad</Message>
    </Class>
    <Method classname="demo.Bad" name="unclosedStream" signature="(Ljava/lang/String;)V" isStatic="true" primary="true">
      <SourceLine classname="demo.Bad" start="8" end="10" startBytecode="0" endBytecode="46" sourcefile="Bad.java" sourcepath="demo/Bad.java"/>
      <Message>In method demo.Bad.unclosedStream(String)</Message>
    </Method>
    <Class classname="java.io.InputStream" role="CLASS_REFTYPE">
      <SourceLine classname="java.io.InputStream" start="61" end="786" sourcefile="InputStream.java" sourcepath="java/io/InputStream.java">
        <Message>At InputStream.java:[lines 61-786]</Message>
      </SourceLine>
      <Message>Reference type java.io.InputStream</Message>
    </Class>
    <Int value="1" role="INT_OBLIGATIONS_REMAINING">
      <Message>1 instances of obligation remaining</Message>
    </Int>
    <SourceLine classname="demo.Bad" primary="true" start="8" end="8" startBytecode="5" endBytecode="5" sourcefile="Bad.java" sourcepath="demo/Bad.java" role="SOURCE_LINE_OBLIGATION_CREATED">
      <Message>Obligation to clean up resource created at Bad.java:[line 8] is not discharged</Message>
    </SourceLine>
    <SourceLine classname="demo.Bad" start="9" end="9" startBytecode="9" endBytecode="9" sourcefile="Bad.java" sourcepath="demo/Bad.java" role="SOURCE_LINE_PATH_CONTINUES">
      <Message>Path continues at Bad.java:[line 9]</Message>
    </SourceLine>
    <SourceLine classname="demo.Bad" start="10" end="10" startBytecode="14" endBytecode="14" sourcefile="Bad.java" sourcepath="demo/Bad.java" role="SOURCE_LINE_PATH_CONTINUES">
      <Message>Path continues at Bad.java:[line 10]</Message>
    </SourceLine>
    <String value="{InputStream x 1}" role="STRING_REMAINING_OBLIGATIONS">
      <Message>Remaining obligations: {InputStream x 1}</Message>
    </String>
  </BugInstance>
  <Errors errors="0" missingClasses="0"></Errors>
</BugCollection>
"#;

#[test]
fn parse_spotbugs_xml_extracts_every_bug_instance() {
    let findings = parse_spotbugs_xml(REAL_SPOTBUGS_REPORT).expect("parses");
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].classname, "demo.Bad");
    assert_eq!(findings[0].priority, 1);
    assert_eq!(
        findings[0].message,
        "Comparison of String parameter using == or != in demo.Bad.compareStrings(String, String)"
    );
}

#[test]
fn parse_spotbugs_xml_picks_the_primary_source_line_not_the_last_direct_child() {
    let findings = parse_spotbugs_xml(REAL_SPOTBUGS_REPORT).expect("parses");
    // The OBL_UNSATISFIED_OBLIGATION finding has three direct-child
    // <SourceLine>s (lines 8, 9, 10) — only the first (line 8) carries
    // primary="true"; a "last direct child" heuristic would wrongly
    // pick line 10.
    assert_eq!(findings[1].classname, "demo.Bad");
    assert_eq!(findings[1].line, 8);
    assert_eq!(findings[1].priority, 2);
}

#[test]
fn parse_spotbugs_xml_ignores_source_lines_nested_inside_class_and_method() {
    let findings = parse_spotbugs_xml(REAL_SPOTBUGS_REPORT).expect("parses");
    // The ES_COMPARING_PARAMETER_STRING_WITH_EQ finding's own primary
    // <SourceLine> (line 13) is a direct child; several other
    // <SourceLine>s nested inside <Class>/<Method>/<Type> (lines 6, 13
    // again, 140) must not be picked up as separate findings or override
    // the real one.
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].line, 13);
}

#[test]
fn spotbugs_severity_maps_high_to_error_and_the_rest_to_warning() {
    assert_eq!(spotbugs_severity(1), ProblemSeverity::Error);
    assert_eq!(spotbugs_severity(2), ProblemSeverity::Warning);
    assert_eq!(spotbugs_severity(3), ProblemSeverity::Warning);
}

#[test]
fn spotbugs_source_file_finds_the_standard_main_layout_path() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src/main/java/demo");
    std::fs::create_dir_all(&src_dir).unwrap();
    let file = src_dir.join("Bad.java");
    std::fs::write(&file, "package demo;\nclass Bad {}\n").unwrap();

    assert_eq!(spotbugs_source_file(dir.path(), "demo.Bad"), Some(file));
}

#[test]
fn spotbugs_source_file_reduces_a_nested_class_to_its_outer_java_file() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src/main/java/demo");
    std::fs::create_dir_all(&src_dir).unwrap();
    let file = src_dir.join("Bad.java");
    std::fs::write(&file, "package demo;\nclass Bad { class Inner {} }\n").unwrap();

    assert_eq!(spotbugs_source_file(dir.path(), "demo.Bad$Inner"), Some(file));
}

#[test]
fn spotbugs_source_file_is_none_when_the_file_does_not_exist() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(spotbugs_source_file(dir.path(), "demo.DoesNotExist"), None);
}

#[test]
fn resolve_findings_resolves_against_the_real_file_and_drops_unresolvable_ones() {
    let dir = tempfile::tempdir().unwrap();
    let src_dir = dir.path().join("src/main/java/demo");
    std::fs::create_dir_all(&src_dir).unwrap();
    let file = src_dir.join("Bad.java");
    std::fs::write(
        &file,
        "package demo;\n\nclass Bad {\n    void m() {\n        int x = 1;\n    }\n}\n",
    )
    .unwrap();

    let findings = vec![
        SpotBugsFinding {
            classname: "demo.Bad".to_string(),
            line: 5,
            priority: 1,
            message: "found".to_string(),
        },
        SpotBugsFinding {
            classname: "demo.Ghost".to_string(),
            line: 1,
            priority: 1,
            message: "unresolvable".to_string(),
        },
    ];
    let resolved = resolve_findings(dir.path(), findings);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].file, file);
    assert_eq!(resolved[0].severity, ProblemSeverity::Error);
    assert_eq!(resolved[0].message, "found");
}
