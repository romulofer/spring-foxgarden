use std::path::PathBuf;

use fg_extension::ProblemSeverity;

use super::*;

#[test]
fn contributes_maven_and_gradle_with_their_own_marker_files() {
    let tools = contributions();
    let maven = tools.iter().find(|t| t.id == MAVEN).unwrap();
    let gradle = tools.iter().find(|t| t.id == GRADLE).unwrap();
    assert_eq!(maven.marker_files, vec!["pom.xml"]);
    assert_eq!(gradle.marker_files, vec!["build.gradle.kts", "build.gradle"]);
}

#[cfg(not(windows))]
#[test]
fn maven_program_prefers_a_real_mvnw_over_the_bare_binary() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mvnw"), "#!/bin/sh\n").unwrap();
    assert_eq!(maven_program(dir.path()), dir.path().join("mvnw"));
}

#[cfg(not(windows))]
#[test]
fn maven_program_falls_back_to_the_bare_binary_with_no_wrapper_present() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(maven_program(dir.path()), PathBuf::from("mvn"));
}

#[cfg(windows)]
#[test]
fn maven_program_prefers_a_real_mvnw_cmd_over_the_bare_binary() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mvnw.cmd"), "@echo off\r\n").unwrap();
    assert_eq!(maven_program(dir.path()), dir.path().join("mvnw.cmd"));
}

#[cfg(windows)]
#[test]
fn maven_program_falls_back_to_the_bare_binary_with_no_wrapper_present() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(maven_program(dir.path()), PathBuf::from("mvn"));
}

#[cfg(windows)]
#[test]
fn maven_program_ignores_a_unix_only_mvnw_with_no_cmd_counterpart() {
    // A wrapper-generated project always ships both `mvnw`/`mvnw.cmd`
    // together, but a hand-rolled or stripped-down one might not —
    // `Command::new`ing the extensionless POSIX script directly would
    // fail outright on Windows (no shebang support), so this must fall
    // back to the bare `mvn` on PATH instead of trying it.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("mvnw"), "#!/bin/sh\n").unwrap();
    assert_eq!(maven_program(dir.path()), PathBuf::from("mvn"));
}

// --- Maven: real `mvn -B compile` output against a deliberately broken
// two-error fixture, captured verbatim (see PLAN.md Track 22 Phase 1) ---

#[test]
fn parses_a_real_maven_compiler_error_line() {
    let line = "[ERROR] /home/user/project/src/main/java/com/example/Foo.java:[5,17] cannot find symbol";
    let problem = parse_output_line(line).unwrap();
    assert_eq!(
        problem.path,
        PathBuf::from("/home/user/project/src/main/java/com/example/Foo.java")
    );
    assert_eq!(problem.line, 5);
    assert_eq!(problem.column, 17);
    assert_eq!(problem.severity, ProblemSeverity::Error);
    assert_eq!(problem.message, "cannot find symbol");
}

#[test]
fn parses_a_real_maven_compiler_warning_line() {
    let line = "[WARNING] /home/user/project/src/main/java/Foo.java:[3,1] some warning text";
    let problem = parse_output_line(line).unwrap();
    assert_eq!(problem.severity, ProblemSeverity::Warning);
    assert_eq!(problem.line, 3);
    assert_eq!(problem.column, 1);
}

#[test]
fn ignores_maven_lines_with_no_source_location() {
    assert!(
        parse_output_line(
            "[ERROR] Failed to execute goal org.apache.maven.plugins:maven-compiler-plugin:3.11.0:compile"
        )
        .is_none()
    );
    assert!(parse_output_line("[ERROR] COMPILATION ERROR : ").is_none());
    assert!(parse_output_line("[ERROR]   symbol:   variable undefinedSymbol").is_none());
}

// --- Gradle: real `gradle --console=plain compileJava` output (plain
// javac shape, no Maven wrapper) ---

#[test]
fn parses_a_real_gradle_javac_error_line() {
    let line = "/home/user/project/src/main/java/com/example/Foo.java:5: error: cannot find symbol";
    let problem = parse_output_line(line).unwrap();
    assert_eq!(
        problem.path,
        PathBuf::from("/home/user/project/src/main/java/com/example/Foo.java")
    );
    assert_eq!(problem.line, 5);
    assert_eq!(problem.column, 1);
    assert_eq!(problem.severity, ProblemSeverity::Error);
    assert_eq!(problem.message, "cannot find symbol");
}

#[test]
fn parses_the_indented_copy_gradles_own_summary_repeats() {
    // The exact 2-space-indented repetition Gradle's own "* What went
    // wrong" section prints for the same error.
    let line = "  /home/user/project/src/main/java/com/example/Foo.java:5: error: cannot find symbol";
    let problem = parse_output_line(line).unwrap();
    assert_eq!(problem.line, 5);
}

#[test]
fn parses_a_real_gradle_javac_warning_line() {
    let line = "/home/user/project/src/main/java/Foo.java:9: warning: some warning text";
    let problem = parse_output_line(line).unwrap();
    assert_eq!(problem.severity, ProblemSeverity::Warning);
}

#[test]
fn ignores_gradle_source_echo_and_caret_lines() {
    assert!(parse_output_line("        int x = undefinedSymbol;").is_none());
    assert!(parse_output_line("                ^").is_none());
    assert!(parse_output_line("  symbol:   variable undefinedSymbol").is_none());
    assert!(parse_output_line("2 errors").is_none());
}

#[test]
fn ignores_plain_prose_lines() {
    assert!(parse_output_line("BUILD FAILED in 3s").is_none());
    assert!(parse_output_line("").is_none());
}

#[test]
fn builds_the_batch_mode_compile_and_test_invocations_each_tool_needs() {
    let dir = tempfile::tempdir().unwrap();
    let maven = command(MAVEN, dir.path(), BuildTask::Compile).unwrap();
    assert_eq!(maven.args, vec!["-B", "compile"]);
    assert_eq!(maven.cwd, dir.path());

    let gradle = command(GRADLE, dir.path(), BuildTask::Test).unwrap();
    assert_eq!(gradle.args, vec!["--console=plain", "test"]);
}

#[test]
fn offers_coverage_for_maven_only() {
    let dir = tempfile::tempdir().unwrap();
    assert!(command(MAVEN, dir.path(), BuildTask::Coverage).is_some());
    assert!(command(GRADLE, dir.path(), BuildTask::Coverage).is_none());
}

#[test]
fn knows_nothing_about_a_build_tool_it_does_not_contribute() {
    let dir = tempfile::tempdir().unwrap();
    assert!(command("bazel", dir.path(), BuildTask::Compile).is_none());
    assert!(classes_dir("bazel", dir.path()).is_none());
}

#[test]
fn points_each_tool_at_its_own_compiled_output_directory() {
    let root = Path::new("/project");
    assert_eq!(classes_dir(MAVEN, root).unwrap(), root.join("target").join("classes"));
    assert_eq!(
        classes_dir(GRADLE, root).unwrap(),
        root.join("build").join("classes").join("java").join("main")
    );
}
