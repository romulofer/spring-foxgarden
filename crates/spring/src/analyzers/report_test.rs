use super::*;
use std::path::Path;

#[test]
fn command_for_binary_wraps_a_jar_in_java_dash_jar() {
    let cmd = command_for_binary(Path::new("/tools/checkstyle-10.26.1-all.jar"));
    assert_eq!(cmd.get_program(), "java");
    let args: Vec<_> = cmd.get_args().collect();
    assert_eq!(args, vec!["-jar", "/tools/checkstyle-10.26.1-all.jar"]);
}

#[test]
fn command_for_binary_runs_a_non_jar_path_directly() {
    let cmd = command_for_binary(Path::new("/tools/pmd-bin-7.26.0/bin/pmd"));
    assert_eq!(cmd.get_program(), "/tools/pmd-bin-7.26.0/bin/pmd");
}

#[test]
fn command_for_binary_runs_the_spotbugs_launcher_script_directly() {
    let cmd = command_for_binary(Path::new("/tools/spotbugs-4.10.3/bin/fb"));
    assert_eq!(cmd.get_program(), "/tools/spotbugs-4.10.3/bin/fb");
}

/// A tool that failed to start prints nothing to stdout; that must come
/// back as its error, not as an empty (clean) report.
#[cfg(unix)]
#[test]
fn a_run_that_printed_no_report_is_an_error_carrying_its_stderr() {
    let output = std::process::Command::new("sh")
        .args(["-c", "echo 'Unable to find: bogus.xml' >&2; exit 254"])
        .output()
        .unwrap();
    let error = report_or_error("Checkstyle", &output).unwrap_err().to_string();
    assert!(error.contains("Checkstyle"), "{error}");
    assert!(error.contains("bogus.xml"), "{error}");
}

/// A non-zero exit with a report is a run that found something.
#[cfg(unix)]
#[test]
fn a_report_is_returned_even_when_the_tool_exits_non_zero() {
    let output = std::process::Command::new("sh")
        .args(["-c", "echo '<checkstyle/>'; exit 3"])
        .output()
        .unwrap();
    assert_eq!(report_or_error("Checkstyle", &output).unwrap().trim(), "<checkstyle/>");
}

