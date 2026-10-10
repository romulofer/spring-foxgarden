//! Checkstyle: `-f xml` reports of style violations in source files.

use std::path::{Path, PathBuf};

use fg_extension::{AnalyzerFinding, AnalyzerRun, ProblemSeverity};
use quick_xml::Reader;
use quick_xml::events::Event;

use super::report::{attr, command_for_binary, report_or_error};

/// One `<error>` entry from a Checkstyle XML report, still in Checkstyle's
/// own line/column terms — not yet a byte range, since that needs the
/// referenced file's actual content (see `line_col_to_byte`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckstyleFinding {
    pub file: PathBuf,
    /// 1-based, as Checkstyle reports it.
    pub line: usize,
    /// 1-based character offset into `line`, as Checkstyle reports it —
    /// `None` for the handful of whole-file checks (e.g. a missing
    /// `package-info.java`) that have no specific column.
    pub column: Option<usize>,
    pub severity: ProblemSeverity,
    pub message: String,
}

/// Parses a Checkstyle XML report (the `-f xml` format) into one
/// `CheckstyleFinding` per `<error>` element, each `<file name="...">`
/// supplying its children's path. Pure/no I/O — verified against real
/// `checkstyle -c sun_checks.xml -f xml` output (see this module's tests),
/// not a guessed schema.
pub fn parse_checkstyle_xml(xml: &str) -> Result<Vec<CheckstyleFinding>, String> {
    let mut reader = Reader::from_str(xml);
    let mut findings = Vec::new();
    let mut current_file: Option<PathBuf> = None;

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Eof => break,
            Event::Start(e) if e.local_name().as_ref() == b"file" => {
                current_file = attr(&e, b"name")?.map(PathBuf::from);
            }
            Event::Empty(e) if e.local_name().as_ref() == b"error" => {
                let file = current_file
                    .clone()
                    .ok_or_else(|| "<error> outside of any <file>".to_string())?;
                let line = attr(&e, b"line")?
                    .ok_or_else(|| "<error> missing a line attribute".to_string())?
                    .parse::<usize>()
                    .map_err(|e| e.to_string())?;
                let column = attr(&e, b"column")?
                    .map(|c| c.parse::<usize>())
                    .transpose()
                    .map_err(|e| e.to_string())?;
                let severity = match attr(&e, b"severity")?.as_deref() {
                    Some("error") => ProblemSeverity::Error,
                    _ => ProblemSeverity::Warning,
                };
                let message = attr(&e, b"message")?.unwrap_or_default();
                findings.push(CheckstyleFinding {
                    file,
                    line,
                    column,
                    severity,
                    message,
                });
            }
            _ => {}
        }
    }

    Ok(findings)
}

/// Runs `binary -c config -f xml <project_root>` and reads the report.
pub fn analyze(run: &AnalyzerRun) -> Result<Vec<AnalyzerFinding>, String> {
    let stdout = run_checkstyle_process(&run.binary, Path::new(&run.config), &run.project_root)?;
    let findings = parse_checkstyle_xml(&stdout)?;
    Ok(findings.into_iter().map(into_finding).collect())
}

fn into_finding(f: CheckstyleFinding) -> AnalyzerFinding {
    AnalyzerFinding {
        file: f.file,
        line: f.line,
        column: f.column,
        end: None,
        severity: f.severity,
        message: f.message,
    }
}

fn run_checkstyle_process(binary: &Path, config: &Path, project_root: &Path) -> Result<String, String> {
    let output = command_for_binary(binary)
        .arg("-c")
        .arg(config)
        .arg("-f")
        .arg("xml")
        .arg(project_root)
        .output()
        .map_err(|e| format!("failed to run Checkstyle: {e}"))?;
    report_or_error("Checkstyle", &output)
}

#[cfg(test)]
#[path = "checkstyle_test.rs"]
mod checkstyle_test;
