//! What the three analyzers' process runners and report readers share.

use std::path::Path;
use std::process::Command;

use quick_xml::events::BytesStart;

/// The XML report a run printed, or an error when it printed none.
///
/// The exit status cannot decide this alone: both Checkstyle and PMD exit
/// non-zero when they *find* something, which is a successful run. What a
/// run that failed to start (bad config path, invalid ruleset, no `java`
/// for a `.jar`) has in common is an empty stdout — and an empty string
/// parses as a valid report with no findings, which would clear every
/// earlier finding and show a clean run instead of the failure.
pub(super) fn report_or_error(tool: &str, output: &std::process::Output) -> Result<String, String> {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    if !stdout.trim().is_empty() {
        return Ok(stdout);
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let detail = stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map_or_else(|| format!("exited with {} and printed no report", output.status), str::to_string);
    Err(format!("{tool}: {detail}"))
}

/// Builds the `Command` to run a configured tool binary at `path`. A bare
/// `.jar` (what an in-app-downloaded Checkstyle install actually is — see
/// since Checkstyle's own GitHub
/// release ships no launcher script, unlike PMD's/SpotBugs') needs `java
/// -jar` wrapped around it to be runnable at all; anything else (a real
/// executable/launcher script, e.g. an apt-installed `/usr/bin/checkstyle`
/// or PMD's/SpotBugs' own `bin/<script>`) is run directly. Requires a JVM
/// on `PATH` for the `.jar` case — not this app's concern to bundle one.
pub(super) fn command_for_binary(path: &Path) -> Command {
    if path.extension().is_some_and(|ext| ext == "jar") {
        let mut cmd = Command::new("java");
        cmd.arg("-jar").arg(path);
        cmd
    } else {
        Command::new(path)
    }
}

pub(super) fn attr(start: &BytesStart<'_>, name: &[u8]) -> Result<Option<String>, String> {
    for a in start.attributes() {
        let a = a.map_err(|e| e.to_string())?;
        if a.key.as_ref() == name {
            return a
                .unescape_value()
                .map(|v| Some(v.into_owned()))
                .map_err(|e| e.to_string());
        }
    }
    Ok(None)
}

#[cfg(test)]
#[path = "report_test.rs"]
mod report_test;
