//! Assembling the real `java` invocation that runs a project's configured run
//! configuration. The tool's own compile command
//! ([`crate::build_tools::command`] with [`fg_extension::BuildTask::Compile`])
//! is expected to have already run and succeeded — this only adds the tool's
//! own *default* compiled-classes output directory to the front of the
//! resolved classpath, it never compiles anything itself.
//!
//! Deliberately **not** `mvn exec:java`/`gradle run`: `gradle run` only
//! exists when the target project applies Gradle's own `application` plugin,
//! which an arbitrary real project — including the editor's own scaffolded
//! Gradle template, which doesn't apply it — has no guarantee of declaring.
//! Launching `java` directly against the already-resolved classpath works
//! regardless of which plugins a project happens to have, the same thing
//! every mainstream IDE's own "Run" already does under the hood. Verified
//! live end-to-end (a real `mvn`/`gradle`-resolved classpath, a real compiled
//! class with a real dependency on it, a real `java -cp ... Main arg1 arg2`
//! launch reading both `System.getenv` and its own `args`), not assumed from
//! either tool's docs.

use std::path::{Path, PathBuf};

use fg_extension::{CommandSpec, RunSpec};

use crate::build_tools::{classes_dir, GRADLE, MAVEN};
use crate::gradle::{gradle_classpaths, GradleClasspath};
use crate::maven::maven_classpath;

/// Resolves `tool_id`'s own real classpath for `project_root` — every jar plus
/// the tool's default compiled-classes output directory, most-specific first.
/// Split out of [`command`] so a caller that needs the raw entry list rather
/// than a shell-ready `-cp` string (a DAP `launch` request, whose
/// `classPaths` argument is a real JSON array) doesn't duplicate this same
/// Maven/Gradle resolution branch a second time.
pub fn runtime_classpath(tool_id: &str, project_root: &Path) -> Option<Result<Vec<PathBuf>, String>> {
    let classes = classes_dir(tool_id, project_root)?;
    match tool_id {
        MAVEN => Some(maven_classpath(project_root).map_err(|e| e.to_string()).map(|mut cp| {
            cp.insert(0, classes);
            cp
        })),
        GRADLE => Some(
            gradle_classpaths(project_root)
                .map_err(|e| e.to_string())
                .and_then(|classpaths| gradle_run_classpath(classpaths, classes)),
        ),
        _ => None,
    }
}

/// The classpath a run of a Gradle build uses. The root project's own when it
/// has a resolvable one. A build whose root applies no JVM plugin (the usual
/// Kotlin Multiplatform layout, with `:shared` and `:composeApp` below an empty
/// root) has none, so the classes and dependencies of every module are joined:
/// the entry point lives in one of them, and the launch does not say which.
fn gradle_run_classpath(classpaths: Vec<GradleClasspath>, default_classes: PathBuf) -> Result<Vec<PathBuf>, String> {
    // The build's own answer for where its classes went; the default directory
    // is only a guess for a script reply that carried none.
    let (outputs, runtime): (Vec<PathBuf>, Vec<PathBuf>) = match classpaths.iter().position(|c| c.path == ":") {
        Some(root) => {
            let root = classpaths.into_iter().nth(root).expect("position found it");
            let outputs = if root.outputs.is_empty() { vec![default_classes] } else { root.outputs };
            (outputs, root.runtime)
        }
        None if classpaths.is_empty() => return Err("no Gradle module with a resolvable classpath".to_string()),
        None => (
            classpaths.iter().flat_map(|c| c.outputs.iter().cloned()).collect(),
            classpaths.iter().flat_map(|c| c.runtime.iter().cloned()).collect(),
        ),
    };
    let mut seen = std::collections::HashSet::new();
    Ok(outputs.into_iter().chain(runtime).filter(|entry| seen.insert(entry.clone())).collect())
}

/// A best-effort classpath for metadata scanning: a plain single-module
/// `pom.xml` resolves directly, but a multi-module *aggregator* `pom.xml`
/// (`<packaging>pom</packaging>`, real `<modules>`) has no dependencies of its
/// own to resolve at all — `mvn dependency:build-classpath` there would
/// resolve nothing useful, so this reads the aggregator's own `<modules>` and
/// resolves each real module directory's classpath instead, unioning the
/// results. Gradle needs no such split: a single invocation at the root
/// already walks the whole multi-module tree.
///
/// A module whose own classpath fails to resolve is skipped rather than
/// failing the whole scan — this serves a background scan nobody asked for,
/// where silence beats an error.
pub fn analysis_classpath(tool_id: &str, project_root: &Path) -> Vec<PathBuf> {
    match tool_id {
        MAVEN => {
            let module_dirs = match std::fs::read_to_string(project_root.join("pom.xml"))
                .ok()
                .and_then(|xml| crate::maven::parse_pom(&xml).ok())
            {
                Some(project) if !project.modules.is_empty() => {
                    project.modules.iter().map(|m| project_root.join(m)).collect()
                }
                _ => vec![project_root.to_path_buf()],
            };
            module_dirs
                .iter()
                .filter_map(|dir| maven_classpath(dir).ok())
                .flatten()
                .collect()
        }
        GRADLE => gradle_classpaths(project_root)
            .map(|classpaths| {
                classpaths
                    .into_iter()
                    .flat_map(|c| c.compile.into_iter().chain(c.runtime))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Assembles the real `java` launch for `run` against `tool_id`'s own
/// resolved classpath, cwd set to `run.working_dir` (or `project_root` when
/// unset).
pub fn command(tool_id: &str, project_root: &Path, run: &RunSpec) -> Option<Result<CommandSpec, String>> {
    let classpath = match runtime_classpath(tool_id, project_root)? {
        Ok(classpath) => classpath,
        Err(error) => return Some(Err(error)),
    };

    Some(Ok(launch_spec(project_root, run, &classpath)))
}

/// The `java` launch itself, split from [`command`] so the argument/
/// environment assembly is testable without a real `mvn`/`gradle` classpath
/// resolution behind it.
fn launch_spec(project_root: &Path, run: &RunSpec, classpath: &[PathBuf]) -> CommandSpec {
    let separator = if cfg!(windows) { ';' } else { ':' };
    let classpath_str = classpath
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(&separator.to_string());

    let cwd = run.working_dir.clone().unwrap_or_else(|| project_root.to_path_buf());
    let mut spec = CommandSpec::new("java", cwd)
        .args(run.vm_args.split_whitespace())
        .arg("-cp")
        .arg(classpath_str)
        .arg(run.entry_point.clone())
        .args(run.program_args.split_whitespace());
    for (key, value) in &run.env {
        spec = spec.env(key.clone(), value.clone());
    }
    spec
}

#[cfg(test)]
#[path = "run_test.rs"]
mod run_test;
