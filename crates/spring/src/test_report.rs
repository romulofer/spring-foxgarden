//! Parsing a test run's own JUnit-XML report: Maven Surefire's `target/
//! surefire-reports/TEST-<FQCN>.xml` and Gradle's `build/test-results/
//! test/TEST-<FQCN>.xml` are, once actually compared side by side against
//! real output (not assumed from either tool's docs), the *same* JUnit-XML
//! schema at the structural level both tools happen to have converged
//! on — one `<testsuite>` per test class, one `<testcase name="" classname=
//! "">` per test method, a nested `<failure>`/`<error>`/`<skipped>` marking
//! anything that isn't a plain pass. `parse_junit_xml` reads either shape
//! with the same code.
//!
//! Two real, found-not-assumed differences between the two tools' own
//! reports, both handled here:
//! - Maven wraps a `<failure>`/`<error>`'s own text in `<![CDATA[...]]>`;
//!   Gradle's is plain element text. `quick_xml` surfaces these as two
//!   different event kinds (`Event::CData` vs. `Event::Text`) — both are
//!   read into the same `detail` field.
//! - Gradle's own `<testcase name="...">` keeps JUnit 5's `()` suffix on a
//!   parameterless test method's display name (`"addIsBroken()"`); Maven's
//!   Surefire report strips it (`"addIsBroken"`). `parse_junit_xml` strips
//!   a trailing `"()"` unconditionally so a case's `name` reads the same
//!   regardless of which tool produced the report.

use std::path::{Path, PathBuf};

use fg_extension::{TestCase, TestFailureLocation, TestOutcome};
use quick_xml::Reader;
use quick_xml::events::Event;

use crate::build_tools::{GRADLE, MAVEN};
use crate::xml::attr;

/// Where `tool` writes its own JUnit-XML reports, relative to
/// `project_root` — verified against a real `mvn -B test`/`gradle
/// --console=plain test` run each (`TEST-<FQCN>.xml` per test class, both
/// tools, not just one directory guessed from the other's).
fn test_report_dir(tool_id: &str, project_root: &Path) -> Option<PathBuf> {
    match tool_id {
        MAVEN => Some(project_root.join("target").join("surefire-reports")),
        GRADLE => Some(project_root.join("build").join("test-results").join("test")),
        _ => None,
    }
}

/// Reads and parses every `TEST-*.xml` report `tool`'s own last test run
/// left behind. A report that can't be read or fails to parse is silently
/// skipped rather than failing the whole scan — a leftover file from a
/// differently-shaped older run shouldn't hide every other class's real
/// results. Empty (no reports directory at all, e.g. the test task never
/// ran) degrades to an empty `Vec`, not an error.
pub fn scan_test_reports(tool_id: &str, project_root: &Path) -> Vec<TestCase> {
    let Some(dir) = test_report_dir(tool_id, project_root) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut cases = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let is_report = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with("TEST-") && n.ends_with(".xml"));
        if !is_report {
            continue;
        }
        if let Ok(xml) = std::fs::read_to_string(&path)
            && let Ok(mut parsed) = parse_junit_xml(&xml)
        {
            cases.append(&mut parsed);
        }
    }
    cases
}

/// Parses one JUnit-XML report (either tool's own shape — see this
/// module's own doc comment) into its `TestCase`s. Pure/no I/O.
pub fn parse_junit_xml(xml: &str) -> Result<Vec<TestCase>, String> {
    let mut reader = Reader::from_str(xml);
    let mut cases = Vec::new();
    let mut pending: Option<TestCase> = None;
    let mut capturing_detail = false;

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Eof => break,
            Event::Start(e) if e.local_name().as_ref() == b"testcase" => {
                let classname = attr(&e, b"classname")?.unwrap_or_default();
                let name = attr(&e, b"name")?.unwrap_or_default();
                pending = Some(TestCase {
                    classname,
                    name: strip_parens(&name),
                    outcome: TestOutcome::Passed,
                    detail: None,
                });
            }
            Event::Empty(e) if e.local_name().as_ref() == b"testcase" => {
                let classname = attr(&e, b"classname")?.unwrap_or_default();
                let name = attr(&e, b"name")?.unwrap_or_default();
                cases.push(TestCase {
                    classname,
                    name: strip_parens(&name),
                    outcome: TestOutcome::Passed,
                    detail: None,
                });
            }
            Event::Start(e) if matches!(e.local_name().as_ref(), b"failure" | b"error") && pending.is_some() => {
                let outcome = if e.local_name().as_ref() == b"failure" {
                    TestOutcome::Failed
                } else {
                    TestOutcome::Errored
                };
                if let Some(case) = pending.as_mut() {
                    case.outcome = outcome;
                    case.detail = Some(String::new());
                }
                capturing_detail = true;
            }
            Event::Empty(e) if matches!(e.local_name().as_ref(), b"failure" | b"error") && pending.is_some() => {
                let outcome = if e.local_name().as_ref() == b"failure" {
                    TestOutcome::Failed
                } else {
                    TestOutcome::Errored
                };
                if let Some(case) = pending.as_mut() {
                    case.outcome = outcome;
                    case.detail = Some(attr(&e, b"message")?.unwrap_or_default());
                }
            }
            Event::Empty(e) if e.local_name().as_ref() == b"skipped" && pending.is_some() => {
                if let Some(case) = pending.as_mut() {
                    case.outcome = TestOutcome::Skipped;
                }
            }
            Event::Text(t) if capturing_detail => {
                let text = t.decode().map_err(|e| e.to_string())?;
                if let Some(case) = pending.as_mut()
                    && let Some(detail) = case.detail.as_mut()
                {
                    detail.push_str(&text);
                }
            }
            Event::CData(t) if capturing_detail => {
                let text = t.decode().map_err(|e| e.to_string())?;
                if let Some(case) = pending.as_mut()
                    && let Some(detail) = case.detail.as_mut()
                {
                    detail.push_str(&text);
                }
            }
            Event::End(e) if matches!(e.local_name().as_ref(), b"failure" | b"error") => {
                capturing_detail = false;
            }
            Event::End(e) if e.local_name().as_ref() == b"testcase" => {
                if let Some(case) = pending.take() {
                    cases.push(case);
                }
            }
            _ => {}
        }
    }

    Ok(cases)
}

fn strip_parens(name: &str) -> String {
    name.strip_suffix("()").unwrap_or(name).to_string()
}

/// The most likely source file for `classname`'s own test — standard
/// Maven/Gradle layout (`src/test/java/<package/path>/<ClassName>.java`),
/// the convention both tools' own scaffolding uses. `None` when that exact file
/// doesn't exist rather than guessing further (a nonstandard source layout,
/// or a generated/kotlin test class) — the caller shows the pass/fail
/// summary regardless, just without a click-to-jump for that one row.
pub fn test_source_file(project_root: &Path, classname: &str) -> Option<PathBuf> {
    let relative = classname.replace('.', "/");
    let candidate = project_root
        .join("src")
        .join("test")
        .join("java")
        .join(format!("{relative}.java"));
    candidate.is_file().then_some(candidate)
}

/// The 1-based line a failing/errored test's own stack trace blames,
/// within its own class — the first stack-frame line naming `classname`'s
/// own simple (unqualified) name followed by `.java:<line>`, e.g. `at
/// com.example.CalcTest.addIsBroken(CalcTest.java:14)`. Verified against
/// real JUnit 5 traces from both a real `mvn test`/`gradle test` failure
/// (both tools produce this exact frame shape — a JUnit/AssertionFailedError
/// stack trace, not a tool-specific format): the *first* such frame is the
/// right one, since every frame above it in the trace belongs to the
/// assertion framework's own internals, not the test itself. `None` when no
/// such frame is found (an exception with no stack trace, or one that never
/// actually re-enters the test's own class — both possible, if rare).
pub fn failure_line(detail: &str, classname: &str) -> Option<usize> {
    let simple_name = classname.rsplit('.').next().unwrap_or(classname);
    let marker = format!("({simple_name}.java:");
    let start = detail.find(&marker)? + marker.len();
    let rest = &detail[start..];
    let end = rest.find(')')?;
    rest[..end].trim().parse().ok()
}

/// Where a failing `case` actually lives, for click-to-jump: the standard
/// Maven/Gradle test source path for its class, plus the line its own stack
/// trace blames (falling back to line 1 when the trace names none).
pub fn failure_location(project_root: &Path, case: &TestCase) -> Option<TestFailureLocation> {
    let path = test_source_file(project_root, &case.classname)?;
    let line = case
        .detail
        .as_deref()
        .and_then(|detail| failure_line(detail, &case.classname))
        .unwrap_or(1);
    Some(TestFailureLocation { path, line })
}

#[cfg(test)]
#[path = "test_report_test.rs"]
mod test_report_test;
