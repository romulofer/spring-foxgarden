//! SpotBugs: `analyze -xml:withMessages` reports of bug patterns found in
//! compiled bytecode.

use std::path::{Path, PathBuf};

use fg_extension::{AnalyzerFinding, AnalyzerRun, ProblemSeverity};
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::report::{attr, command_for_binary};

/// Captures `<Class classname="..." primary="true">`/`<SourceLine
/// start="..." primary="true">` into `classname`/`line` when `e` is one of
/// those two tags and carries `primary="true"` — shared by
/// `parse_spotbugs_xml`'s `Start`/`Empty` arms, both of which need the
/// exact same check at depth `0`.
fn capture_if_primary(
    e: &BytesStart<'_>,
    classname: &mut Option<String>,
    line: &mut Option<usize>,
) -> Result<(), String> {
    match e.local_name().as_ref() {
        b"Class" if attr(e, b"primary")?.as_deref() == Some("true") => {
            *classname = attr(e, b"classname")?;
        }
        b"SourceLine" if attr(e, b"primary")?.as_deref() == Some("true") => {
            *line = attr(e, b"start")?.and_then(|s| s.parse().ok());
        }
        _ => {}
    }
    Ok(())
}

/// One `<BugInstance>` entry from a SpotBugs XML report, already reduced to
/// what a squiggle needs: `classname` is the bug's own *primary* `<Class>`
/// (`classname="..." primary="true"` — verified against a real report to be
/// present regardless of bug shape, whether the actual finding site is a
/// class-, method-, or field-level detector), `line` is the primary
/// `<SourceLine>`'s own `start` attribute (SpotBugs reports no column at
/// all — bytecode has no character offsets to report), and `priority` is
/// SpotBugs' own 1 (High) through at least 3 (Low) scale, read directly off
/// `<BugInstance priority="...">`. Unlike Checkstyle's/PMD's findings,
/// there's no real file path here yet — SpotBugs only knows a class name
/// and a bytecode-debug-info source filename, not where that source lives
/// on disk; resolving that is `spotbugs_source_file`'s job, done separately
/// since it needs a `project_root` this type has no reason to carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotBugsFinding {
    pub classname: String,
    pub line: usize,
    pub priority: u8,
    pub message: String,
}

/// SpotBugs priority 1 (`High`) reads as `Error`; 2 (`Normal`) and lower
/// (`Low`, `Experimental`, ...) as `Warning`. Same "this codebase's own
/// threshold, not a documented convention of the tool itself" judgment call
/// `pmd_severity` already makes, chosen for the same reason: a 2/`Normal`
/// finding (e.g. this module's own real-report fixture's "may fail to close
/// stream") not reading as a hard error by default felt like the right
/// starting point.
fn spotbugs_severity(priority: u8) -> ProblemSeverity {
    if priority <= 1 {
        ProblemSeverity::Error
    } else {
        ProblemSeverity::Warning
    }
}

/// The most likely source file for a bug's own `classname` — standard
/// Maven/Gradle layout (`src/main/java/<package/path>/<ClassName>.java`),
/// the same convention `test_report::test_source_file` already established
/// for `src/test/java`. A nested/inner/anonymous class (`Outer$Inner`,
/// `Outer$1`) is reduced to its outer class first — Java always compiles
/// those into the *outer* class's own `.java` file, never their own.
/// `None` when that exact file doesn't exist, same "don't guess further"
/// degrade `test_source_file` already uses.
fn spotbugs_source_file(project_root: &Path, classname: &str) -> Option<PathBuf> {
    let outer = classname.split('$').next().unwrap_or(classname);
    let relative = outer.replace('.', "/");
    let candidate = project_root
        .join("src")
        .join("main")
        .join("java")
        .join(format!("{relative}.java"));
    candidate.is_file().then_some(candidate)
}

/// Runs `binary analyze -xml:withMessages -output <tmp> classes_dir` and
/// reads the report, resolving each finding's class to its source file under
/// `project_root`. A finding whose class can't be mapped back to a real file
/// on disk (a nonstandard source layout, a generated/synthetic class with no
/// `.java` of its own) is silently dropped rather than failing the whole
/// batch.
///
/// Unlike Checkstyle/PMD — whose exit codes double as their violation count,
/// so success is judged by whether stdout parses — SpotBugs' exit code *is* a
/// real success/failure signal: verified live (a real "no files to analyze"
/// run against a nonexistent classes directory) exits `1` with a Java stack
/// trace on stderr and no report written at all, while a real run with
/// findings — or with none — both exit `0`.
pub fn analyze(run: &AnalyzerRun) -> Result<Vec<AnalyzerFinding>, String> {
    let classes_dir = run
        .classes_dir
        .as_deref()
        .ok_or_else(|| "SpotBugs needs the project's compiled classes".to_string())?;
    let xml = run_spotbugs_process(&run.binary, classes_dir)?;
    let findings = parse_spotbugs_xml(&xml)?;
    Ok(resolve_findings(&run.project_root, findings))
}

/// Maps each finding's class to its source file under `project_root`,
/// dropping the ones that don't resolve.
fn resolve_findings(project_root: &Path, findings: Vec<SpotBugsFinding>) -> Vec<AnalyzerFinding> {
    findings
        .into_iter()
        .filter_map(|f| {
            spotbugs_source_file(project_root, &f.classname).map(|file| AnalyzerFinding {
                file,
                line: f.line,
                column: None,
                end: None,
                severity: spotbugs_severity(f.priority),
                message: f.message,
            })
        })
        .collect()
}

/// Spawns a real SpotBugs `analyze` run against `classes_dir`, writing its
/// XML report to a process-scoped temp file (`-output`, not stdout — the
/// same "a real file on disk, not piped stdout" shape `maven_classpath`'s
/// own `-Dmdep.outputFile` already established, and for the same reason:
/// the report format wasn't designed to also be a clean stdout stream) and
/// reading it back once the process exits successfully. `-xml:withMessages`
/// (not bare `-xml`) is required for a `<LongMessage>` to be present at all
/// — verified live; the bare `-xml` form omits it entirely, which would
/// leave every `Diagnostic` with no message text.
fn run_spotbugs_process(binary: &Path, classes_dir: &Path) -> Result<String, String> {
    let output_file = std::env::temp_dir().join(format!("foxgarden-spotbugs-report-{}.xml", std::process::id()));

    let result = command_for_binary(binary)
        .arg("analyze")
        .arg("-xml:withMessages")
        .arg("-output")
        .arg(&output_file)
        .arg(classes_dir)
        .output();
    let result = (|| -> Result<String, String> {
        let output = result.map_err(|e| format!("failed to run SpotBugs: {e}"))?;
        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).into_owned());
        }
        std::fs::read_to_string(&output_file)
            .map_err(|e| format!("SpotBugs exited successfully but its own -output report was never written: {e}"))
    })();
    let _ = std::fs::remove_file(&output_file);
    result
}

/// Parses a SpotBugs XML report (the `-xml:withMessages` format) into one
/// `SpotBugsFinding` per `<BugInstance>`. Pure/no I/O — verified against a
/// real `fb analyze -xml:withMessages` run (SpotBugs 4.10.3, see this
/// module's tests), not a guessed schema. Unlike Checkstyle's/PMD's flat
/// finding shape, a `<BugInstance>` nests several *other* elements
/// (`<Class>`, `<Method>`, `<Type>`, `<Int>`, `<String>`, ...) that carry
/// their *own* nested `<SourceLine>`/`<Message>` children describing
/// secondary/contextual locations, not the bug's own primary one — a naive
/// "first `<SourceLine>` seen" or "last direct child" (an earlier, *wrong*
/// guess this project made before checking a real report side by side: a
/// bug with more than one direct-child `<SourceLine>` — e.g. an
/// `OBL_UNSATISFIED_OBLIGATION` finding's own "obligation created" plus
/// "path continues" trail — has its real primary line *first*, not last)
/// would as often as not pick a wrong location. This tracks nesting depth
/// relative to the current `<BugInstance>` and only accepts a `<Class>`/
/// `<SourceLine>` reading at depth `0` (a *direct* child) that also carries
/// `primary="true"` — the one attribute SpotBugs itself uses to mark which
/// of several same-shaped elements is the real one.
pub fn parse_spotbugs_xml(xml: &str) -> Result<Vec<SpotBugsFinding>, String> {
    let mut reader = Reader::from_str(xml);
    let mut findings = Vec::new();

    let mut in_bug = false;
    let mut depth: i32 = 0;
    let mut priority: Option<u8> = None;
    let mut classname: Option<String> = None;
    let mut line: Option<usize> = None;
    let mut message = String::new();
    let mut in_long_message = false;

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Eof => break,

            Event::Start(e) if !in_bug && e.local_name().as_ref() == b"BugInstance" => {
                in_bug = true;
                depth = 0;
                priority = Some(
                    attr(&e, b"priority")?
                        .ok_or_else(|| "<BugInstance> missing priority".to_string())?
                        .parse::<u8>()
                        .map_err(|e| e.to_string())?,
                );
                classname = None;
                line = None;
                message.clear();
            }

            Event::Start(e) if in_bug => {
                if depth == 0 {
                    if e.local_name().as_ref() == b"LongMessage" {
                        in_long_message = true;
                    }
                    capture_if_primary(&e, &mut classname, &mut line)?;
                }
                depth += 1;
            }

            Event::Empty(e) if in_bug && depth == 0 => {
                capture_if_primary(&e, &mut classname, &mut line)?;
            }

            Event::Text(t) if in_long_message => {
                let text = t.decode().map_err(|e| e.to_string())?;
                let unescaped = quick_xml::escape::unescape(&text).map_err(|e| e.to_string())?;
                message.push_str(unescaped.trim());
            }

            Event::End(e) if in_bug => {
                depth -= 1;
                if e.local_name().as_ref() == b"LongMessage" {
                    in_long_message = false;
                }
                if e.local_name().as_ref() == b"BugInstance" {
                    in_bug = false;
                    if let (Some(classname), Some(line)) = (classname.take(), line.take()) {
                        findings.push(SpotBugsFinding {
                            classname,
                            line,
                            priority: priority.expect("set when entering <BugInstance>"),
                            message: message.clone(),
                        });
                    }
                }
            }

            _ => {}
        }
    }

    Ok(findings)
}

#[cfg(test)]
#[path = "spotbugs_test.rs"]
mod spotbugs_test;
