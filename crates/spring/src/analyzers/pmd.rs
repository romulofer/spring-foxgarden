//! PMD: `check -f xml` reports of rule violations in source files.

use std::path::{Path, PathBuf};

use fg_extension::{AnalyzerFinding, AnalyzerRun, ProblemSeverity};
use quick_xml::Reader;
use quick_xml::events::Event;

use super::report::{attr, command_for_binary, report_or_error};

/// One `<violation>` entry from a PMD XML report, still in PMD's own
/// line/column terms. Unlike Checkstyle, PMD reports a real begin/end
/// range (both ends inclusive character columns — verified against a real
/// `pmd check -f xml` run, see this module's tests) rather than a single
/// point, and has no built-in error/warning distinction of its own, just a
/// 1 (highest) through 5 (lowest) `priority` — `pmd_severity` maps that
/// onto this codebase's binary `Severity` as a judgment call, not something
/// PMD itself defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmdFinding {
    pub file: PathBuf,
    pub begin_line: usize,
    pub begin_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub priority: u8,
    pub message: String,
}

/// PMD priority 1 (`HIGH`) and 2 (`MEDIUM_HIGH`) read as `Error`; 3
/// (`MEDIUM`) through 5 (`LOW`) as `Warning`. PMD itself has no
/// error/warning concept — this is this codebase's own threshold, not a
/// documented PMD convention, chosen because "compare objects with
/// reference equality" (a 3/`MEDIUM_HIGH`-adjacent style of finding) not
/// showing as a hard error in the common quickstart ruleset felt like the
/// right default; revisit if real usage disagrees.
fn pmd_severity(priority: u8) -> ProblemSeverity {
    if priority <= 2 {
        ProblemSeverity::Error
    } else {
        ProblemSeverity::Warning
    }
}

/// Parses a PMD XML report (the `-f xml` format) into one `PmdFinding` per
/// `<violation>` element, each `<file name="...">` supplying its
/// children's path. Pure/no I/O — verified against real
/// `pmd check -R rulesets/java/quickstart.xml -f xml` output (see this
/// module's tests), not a guessed schema. Unlike Checkstyle's `<error/>`
/// (self-closing, message as an attribute), PMD's `<violation>` wraps its
/// message as element text content, so this tracks an in-progress
/// violation's attributes across `Start`/`Text`/`End` rather than reading
/// everything off one `Empty` event.
pub fn parse_pmd_xml(xml: &str) -> Result<Vec<PmdFinding>, String> {
    let mut reader = Reader::from_str(xml);
    let mut findings = Vec::new();
    let mut current_file: Option<PathBuf> = None;
    let mut pending: Option<PmdFinding> = None;

    loop {
        match reader.read_event().map_err(|e| e.to_string())? {
            Event::Eof => break,
            Event::Start(e) if e.local_name().as_ref() == b"file" => {
                current_file = attr(&e, b"name")?.map(PathBuf::from);
            }
            Event::Start(e) if e.local_name().as_ref() == b"violation" => {
                let file = current_file
                    .clone()
                    .ok_or_else(|| "<violation> outside of any <file>".to_string())?;
                let usize_attr = |name: &[u8]| -> Result<usize, String> {
                    attr(&e, name)?
                        .ok_or_else(|| format!("<violation> missing {}", String::from_utf8_lossy(name)))?
                        .parse::<usize>()
                        .map_err(|e| e.to_string())
                };
                let priority = attr(&e, b"priority")?
                    .ok_or_else(|| "<violation> missing priority".to_string())?
                    .parse::<u8>()
                    .map_err(|e| e.to_string())?;
                pending = Some(PmdFinding {
                    file,
                    begin_line: usize_attr(b"beginline")?,
                    begin_column: usize_attr(b"begincolumn")?,
                    end_line: usize_attr(b"endline")?,
                    end_column: usize_attr(b"endcolumn")?,
                    priority,
                    message: String::new(),
                });
            }
            Event::Text(t) if pending.is_some() => {
                let text = t.decode().map_err(|e| e.to_string())?;
                let unescaped = quick_xml::escape::unescape(&text).map_err(|e| e.to_string())?;
                if let Some(finding) = pending.as_mut() {
                    finding.message.push_str(unescaped.trim());
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"violation" => {
                let finding = pending
                    .take()
                    .ok_or_else(|| "</violation> without a matching start".to_string())?;
                findings.push(finding);
            }
            _ => {}
        }
    }

    Ok(findings)
}

/// Runs `binary check -d project_root -R ruleset -f xml --no-cache` and
/// reads the report. `--no-cache` always disabled: PMD's incremental-analysis
/// cache is meant for repeated runs against an unchanged codebase, which
/// doesn't fit a batch job run on demand, and a stale cache silently
/// under-reporting would be a worse failure mode than the extra cost of a
/// fresh run every time.
pub fn analyze(run: &AnalyzerRun) -> Result<Vec<AnalyzerFinding>, String> {
    let stdout = run_pmd_process(&run.binary, &run.config, &run.project_root)?;
    let findings = parse_pmd_xml(&stdout)?;
    Ok(findings.into_iter().map(into_finding).collect())
}

/// PMD reports a real begin/end range, both ends inclusive.
fn into_finding(f: PmdFinding) -> AnalyzerFinding {
    AnalyzerFinding {
        file: f.file,
        line: f.begin_line,
        column: Some(f.begin_column),
        end: Some((f.end_line, f.end_column)),
        severity: pmd_severity(f.priority),
        message: f.message,
    }
}

fn run_pmd_process(binary: &Path, ruleset: &str, project_root: &Path) -> Result<String, String> {
    let output = command_for_binary(binary)
        .arg("check")
        .arg("-d")
        .arg(project_root)
        .arg("-R")
        .arg(ruleset)
        .arg("-f")
        .arg("xml")
        .arg("--no-cache")
        .output()
        .map_err(|e| format!("failed to run PMD: {e}"))?;
    report_or_error("PMD", &output)
}

#[cfg(test)]
#[path = "pmd_test.rs"]
mod pmd_test;
