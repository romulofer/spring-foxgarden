//! Maven and Gradle as contributed build tools: how each one is detected,
//! what process performs each of the core's standard tasks, where it drops
//! compiled output, and what its diagnostic output looks like.
//!
//! Spawning and streaming the process stays on the editor's side — this only
//! describes what to run, which is why everything here returns a
//! [`CommandSpec`] rather than a live `Command`.

use std::path::{Path, PathBuf};

use fg_extension::{BuildProblem, BuildTask, BuildToolContribution, CommandSpec, ProblemSeverity};

use crate::coverage;

/// The build tool ids this extension registers. Public because a caller that
/// needs to look one up by id should use these rather than a string literal.
pub const MAVEN: &str = "maven";
pub const GRADLE: &str = "gradle";

/// What the registry detects projects with: the same `pom.xml` vs
/// `build.gradle[.kts]` file-presence check the editor's core used to
/// hardcode, now stated as data.
pub fn contributions() -> Vec<BuildToolContribution> {
    vec![
        BuildToolContribution {
            id: MAVEN.to_string(),
            display_name: "Maven".to_string(),
            marker_files: vec!["pom.xml".to_string()],
        },
        BuildToolContribution {
            id: GRADLE.to_string(),
            display_name: "Gradle".to_string(),
            marker_files: vec!["build.gradle.kts".to_string(), "build.gradle".to_string()],
        },
    ]
}

/// Prefers a project-pinned `mvnw`/`mvnw.cmd` over a bare `mvn` on `PATH` —
/// unlike `maven::maven_classpath` (always a bare `mvn`, a read-only
/// classpath dump where the exact Maven version doesn't matter), mirroring
/// `gradle_program`'s own "prefer what the project actually specifies"
/// wrapper-first choice, now that this is a real build. Two platform-gated
/// bodies, not one that picks the extension internally: `mvnw` is a POSIX
/// shell script with no PE header, so launching it directly on Windows fails
/// outright (`CreateProcess` has no shebang support) rather than falling
/// through to the bare-`mvn` branch — the wrapper generator always emits
/// *both* `mvnw`/`mvnw.cmd`, so picking the right one per-OS, the same split
/// `../references/java`'s own `task_helper::build_tool::which_wrapper` uses,
/// is correct rather than a guess.
#[cfg(windows)]
pub(crate) fn maven_program(project_root: &Path) -> PathBuf {
    let wrapper = project_root.join("mvnw.cmd");
    if wrapper.is_file() { wrapper } else { PathBuf::from("mvn") }
}

#[cfg(not(windows))]
pub(crate) fn maven_program(project_root: &Path) -> PathBuf {
    let wrapper = project_root.join("mvnw");
    if wrapper.is_file() { wrapper } else { PathBuf::from("mvn") }
}

/// Picks `<project_root>/gradlew`/`gradlew.bat` when present (a project's own
/// pinned wrapper — the right version to actually build with) over a bare
/// `gradle` on `PATH`. Platform-split for the same reason `maven_program` is.
#[cfg(windows)]
pub(crate) fn gradle_program(project_root: &Path) -> PathBuf {
    let wrapper = project_root.join("gradlew.bat");
    if wrapper.is_file() {
        wrapper
    } else {
        PathBuf::from("gradle")
    }
}

#[cfg(not(windows))]
pub(crate) fn gradle_program(project_root: &Path) -> PathBuf {
    let wrapper = project_root.join("gradlew");
    if wrapper.is_file() {
        wrapper
    } else {
        PathBuf::from("gradle")
    }
}

/// The process for one standard task. Verified against real `mvn 3.9.3`/
/// `gradle 9.6.1` runs, not assumed: `-B` (Maven's own batch/non-interactive
/// mode) and `--console=plain` (Gradle) both keep the output free of the
/// interactive progress noise a piped, non-TTY stdout doesn't automatically
/// get stripped of in every configuration.
///
/// `None` for Gradle + [`BuildTask::Coverage`]: coverage is JaCoCo invoked as
/// a bare Maven plugin goal (see [`crate::coverage`]), which has no Gradle
/// counterpart that works without editing the project's own build script.
pub fn command(tool_id: &str, project_root: &Path, task: BuildTask) -> Option<CommandSpec> {
    match (tool_id, task) {
        (MAVEN, BuildTask::Compile) => Some(
            CommandSpec::new(maven_program(project_root), project_root)
                .arg("-B")
                .arg("compile"),
        ),
        (MAVEN, BuildTask::Test) => Some(
            CommandSpec::new(maven_program(project_root), project_root)
                .arg("-B")
                .arg("test"),
        ),
        (MAVEN, BuildTask::Coverage) => Some(coverage::command(project_root)),
        (GRADLE, BuildTask::Compile) => Some(
            CommandSpec::new(gradle_program(project_root), project_root)
                .arg("--console=plain")
                .arg(gradle_compile_task(project_root)),
        ),
        (GRADLE, BuildTask::Test) => Some(
            CommandSpec::new(gradle_program(project_root), project_root)
                .arg("--console=plain")
                .arg("test"),
        ),
        _ => None,
    }
}

/// The process running only the tests `filter` names — `pkg.Class`, or
/// `pkg.Class#method` for one — through the same wrapper the whole-project
/// test task uses. `None` for a tool this extension does not own.
///
/// Maven's `-Dtest` takes `Class#method`, and Surefire is told not to fail
/// where a module has no match, since a multi-module run only matches in one of
/// them. Gradle's `--tests` writes the method after a dot, and a build with a
/// Kotlin Multiplatform JVM target has `jvmTest` where others have `test`.
pub fn test_command(tool_id: &str, project_root: &Path, filter: &str) -> Option<CommandSpec> {
    match tool_id {
        // Surefire's method patterns have no room for the spaces of a Kotlin
        // backtick name, and take `*` for any run of characters.
        MAVEN => Some(
            CommandSpec::new(maven_program(project_root), project_root)
                .arg("-B")
                .arg(format!("-Dtest={}", filter.replace(' ', "*")))
                .arg("-Dsurefire.failIfNoSpecifiedTests=false")
                .arg("-DfailIfNoSpecifiedTests=false")
                .arg("test"),
        ),
        GRADLE => {
            let task = if multiplatform_jvm_module_below(project_root, 0) { "jvmTest" } else { "test" };
            Some(
                CommandSpec::new(gradle_program(project_root), project_root)
                    .arg("--console=plain")
                    .arg(task)
                    .arg("--tests")
                    .arg(filter.replace('#', ".")),
            )
        }
        _ => None,
    }
}

/// The Gradle task that compiles the main code: `compileJava`, except in a
/// build with a Kotlin Multiplatform module that has a JVM target, which has no
/// such task (`compileJava` is ambiguous there) and compiles through
/// `jvmMainClasses`. Selected by name from the root, that task runs in every
/// module that has it, so a root applying no plugin over `:shared` and
/// `:composeApp` compiles both.
fn gradle_compile_task(project_root: &Path) -> &'static str {
    if multiplatform_jvm_module_below(project_root, 0) { "jvmMainClasses" } else { "compileJava" }
}

/// How far below the root a Gradle module is looked for, as in `:libs:core`.
const MODULE_SEARCH_DEPTH: usize = 2;

/// Directories that are never modules.
const NOT_MODULES: [&str; 7] = [".git", ".gradle", ".idea", "build", "node_modules", "out", "target"];

/// Whether `dir`, or a module below it, applies the multiplatform plugin and
/// declares a JVM target.
fn multiplatform_jvm_module_below(dir: &Path, depth: usize) -> bool {
    if is_multiplatform_jvm_build(dir) {
        return true;
    }
    if depth == MODULE_SEARCH_DEPTH {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return false };
    entries.flatten().map(|entry| entry.path()).filter(|path| path.is_dir()).any(|path| {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        !name.starts_with('.') && !NOT_MODULES.contains(&name) && multiplatform_jvm_module_below(&path, depth + 1)
    })
}

fn is_multiplatform_jvm_build(dir: &Path) -> bool {
    let build_file = ["build.gradle.kts", "build.gradle"]
        .iter()
        .find_map(|name| std::fs::read_to_string(dir.join(name)).ok())
        .unwrap_or_default();
    let code: Vec<&str> = build_file.lines().map(str::trim).filter(|line| !line.starts_with("//")).collect();
    code.iter().any(|line| line.contains("multiplatform")) && code.iter().any(|line| declares_jvm_target(line))
}

/// Whether `line` calls the `jvm` target function (`jvm()`, `jvm("desktop")`,
/// `jvm { }`), not `jvmToolchain(...)` or a name that merely ends in `jvm`.
fn declares_jvm_target(line: &str) -> bool {
    line.match_indices("jvm").any(|(at, _)| {
        let before_ok = line[..at].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || c == '_' || c == '.'));
        before_ok && line[at + 3..].trim_start().starts_with(['(', '{'])
    })
}

/// Each tool's own default compiled-classes output directory. Not verified to
/// exist on disk — a build that hasn't run yet (or that failed) simply means
/// a missing directory; callers decide what that means for them.
pub fn classes_dir(tool_id: &str, project_root: &Path) -> Option<PathBuf> {
    match tool_id {
        MAVEN => Some(project_root.join("target").join("classes")),
        GRADLE => Some(project_root.join("build").join("classes").join("java").join("main")),
        _ => None,
    }
}

/// Parses one line of `mvn`/`gradle` build output into a [`BuildProblem`], if
/// that line names a compiler diagnostic. Two independent shapes, verified
/// against real `mvn 3.9.3 -B compile`/`gradle 9.6.1 --console=plain
/// compileJava` runs against a deliberately broken two-error fixture (not
/// assumed from either tool's docs):
///
/// - Maven wraps javac's own line in `[LEVEL] `: `[ERROR] /abs/path/
///   Foo.java:[12,34] message text` (also `[WARNING]`) — emitted on
///   **stdout**.
/// - Gradle's `compileJava` prints javac's own unwrapped format instead:
///   `/abs/path/Foo.java:12: error: message text` (also `: warning:`), also
///   indented by 2 spaces in the "What went wrong" summary that repeats it —
///   emitted on **stderr**, unlike Maven's own shape. No column at all here
///   (javac instead points at it with a `^` under a repeated source line on
///   the two lines that follow); `column` is reported as `1` rather than
///   parsing the caret line, since landing on the right *line* is what
///   click-to-jump needs, not the exact column.
///
/// Both shapes are tried for either tool. A wrapper script, a plugin, or a
/// `mvn` invocation that happens to print raw javac output is common enough
/// that keying each parser to one tool id would just lose diagnostics.
pub fn parse_output_line(line: &str) -> Option<BuildProblem> {
    parse_maven_line(line).or_else(|| parse_javac_line(line))
}

fn parse_maven_line(line: &str) -> Option<BuildProblem> {
    let (severity, rest) = if let Some(rest) = line.strip_prefix("[ERROR] ") {
        (ProblemSeverity::Error, rest)
    } else {
        let rest = line.strip_prefix("[WARNING] ")?;
        (ProblemSeverity::Warning, rest)
    };

    let marker = ".java:[";
    let marker_at = rest.find(marker)?;
    let path = &rest[..marker_at + ".java".len()];
    if path.is_empty() {
        return None;
    }
    let after = &rest[marker_at + marker.len()..]; // "12,34] message"
    let close_at = after.find(']')?;
    let (line_str, col_str) = after[..close_at].split_once(',')?;
    let line_no: usize = line_str.trim().parse().ok()?;
    let col_no: usize = col_str.trim().parse().ok()?;
    let message = after[close_at + 1..].trim();
    if message.is_empty() {
        return None;
    }
    Some(BuildProblem {
        path: PathBuf::from(path),
        line: line_no,
        column: col_no,
        severity,
        message: message.to_string(),
    })
}

fn parse_javac_line(line: &str) -> Option<BuildProblem> {
    let trimmed = line.trim_start();
    let marker = ".java:";
    let marker_at = trimmed.find(marker)?;
    let path = &trimmed[..marker_at + ".java".len()];
    // A real path never contains a space or an early ':' — guards against
    // misreading an unrelated prose line that merely happens to contain the
    // substring ".java:" somewhere past its start.
    if path.is_empty() || path.contains(' ') || path.contains(':') {
        return None;
    }
    let after = &trimmed[marker_at + marker.len()..]; // "12: error: message"
    let (line_str, rest) = after.split_once(':')?;
    let line_no: usize = line_str.trim().parse().ok()?;
    let rest = rest.trim_start();
    let (severity, message) = if let Some(m) = rest.strip_prefix("error:") {
        (ProblemSeverity::Error, m)
    } else {
        let m = rest.strip_prefix("warning:")?;
        (ProblemSeverity::Warning, m)
    };
    let message = message.trim();
    if message.is_empty() {
        return None;
    }
    Some(BuildProblem {
        path: PathBuf::from(path),
        line: line_no,
        column: 1,
        severity,
        message: message.to_string(),
    })
}

#[cfg(test)]
#[path = "build_tools_test.rs"]
mod build_tools_test;
