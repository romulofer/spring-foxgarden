use super::*;

use fg_extension::ScaffoldSpec;

fn sample() -> ScaffoldSpec {
    ScaffoldSpec {
        namespace: "com.example.app".to_string(),
        name: "my-app".to_string(),
        runtime_version: Some(17),
        build_tool_id: MAVEN.to_string(),
        language_id: JAVA.to_string(),
    }
}

/// Unwrapping is the point in every test below: these are the four targets
/// this extension declares it scaffolds, so `None` would be the bug.
fn files_for(spec: &ScaffoldSpec) -> Vec<(PathBuf, String)> {
    scaffold_files(spec).expect("a contributed target scaffolds")
}

#[test]
fn scaffold_files_returns_the_expected_paths() {
    let files = files_for(&sample());
    let paths: Vec<&Path> = files.iter().map(|(p, _)| p.as_path()).collect();
    assert_eq!(
        paths,
        vec![
            Path::new("pom.xml"),
            Path::new("src/main/java/com/example/app/Main.java"),
            Path::new(".gitignore"),
        ]
    );
}

#[test]
fn pom_xml_states_the_requested_release_group_and_artifact() {
    let files = files_for(&sample());
    let (_, pom) = &files[0];
    assert!(
        pom.contains("<maven.compiler.release>17</maven.compiler.release>"),
        "{pom}"
    );
    assert!(pom.contains("<groupId>com.example.app</groupId>"), "{pom}");
    assert!(pom.contains("<artifactId>my-app</artifactId>"), "{pom}");
}

#[test]
fn pom_xml_release_reads_back_through_java_release_detect_at_the_requested_value() {
    // Regression: a scaffolded pom.xml has to be readable by this
    // project's own detector (`PLAN.md` Track 29 Phase 2), or a
    // scaffolded project would silently get analyzed at jdt.ls' own
    // default release instead of the one the wizard promised.
    let spec = ScaffoldSpec {
        runtime_version: Some(8),
        ..sample()
    };
    let (_, pom) = &files_for(&spec)[0];
    assert_eq!(
        crate::java_release::release_from_pom(pom).map(|(major, _)| major),
        Some(8)
    );
}

#[test]
fn main_java_declares_the_package_from_the_group_id_and_a_runnable_main() {
    let files = files_for(&sample());
    let (_, main) = &files[1];
    assert!(main.starts_with("package com.example.app;\n\n"), "{main}");
    assert!(main.contains("public class Main"), "{main}");
    assert!(main.contains("public static void main(String[] args)"), "{main}");
}

#[test]
fn a_single_segment_group_id_still_produces_a_valid_package_and_path() {
    let spec = ScaffoldSpec {
        namespace: "app".to_string(),
        ..sample()
    };
    let files = files_for(&spec);
    assert_eq!(files[1].0, Path::new("src/main/java/app/Main.java"));
    assert!(files[1].1.starts_with("package app;\n\n"));
}

fn gradle_sample() -> ScaffoldSpec {
    ScaffoldSpec {
        build_tool_id: GRADLE.to_string(),
        ..sample()
    }
}

#[test]
fn gradle_scaffold_files_returns_the_expected_paths() {
    let files = files_for(&gradle_sample());
    let paths: Vec<&Path> = files.iter().map(|(p, _)| p.as_path()).collect();
    assert_eq!(
        paths,
        vec![
            Path::new("settings.gradle.kts"),
            Path::new("build.gradle.kts"),
            Path::new("src/main/java/com/example/app/Main.java"),
            Path::new(".gitignore"),
        ]
    );
}

#[test]
fn gradle_settings_names_the_root_project_after_the_artifact_id() {
    let files = files_for(&gradle_sample());
    let (_, settings) = &files[0];
    assert_eq!(settings, "rootProject.name = \"my-app\"\n");
}

#[test]
fn gradle_build_states_the_requested_release_group_and_main_class() {
    let files = files_for(&gradle_sample());
    let (_, build) = &files[1];
    assert!(
        build.contains("languageVersion = JavaLanguageVersion.of(17)"),
        "{build}"
    );
    assert!(build.contains(r#"group = "com.example.app""#), "{build}");
    assert!(build.contains(r#"mainClass = "com.example.app.Main""#), "{build}");
}

#[test]
fn gradle_build_release_reads_back_through_java_release_detect_at_the_requested_value() {
    // Same regression as Maven's own equivalent test: a scaffolded
    // build.gradle.kts has to be readable by java_release::detect at
    // exactly the release the wizard promised.
    let spec = ScaffoldSpec {
        runtime_version: Some(21),
        ..gradle_sample()
    };
    let (_, build) = &files_for(&spec)[1];
    assert_eq!(
        crate::java_release::release_from_gradle(build).map(|(major, _)| major),
        Some(21)
    );
}

#[test]
fn a_single_segment_group_id_still_produces_a_qualified_main_class() {
    let spec = ScaffoldSpec {
        namespace: "app".to_string(),
        ..gradle_sample()
    };
    let files = files_for(&spec);
    let (_, build) = &files[1];
    assert!(build.contains(r#"mainClass = "app.Main""#), "{build}");
}

fn kotlin_maven_sample() -> ScaffoldSpec {
    ScaffoldSpec {
        language_id: KOTLIN.to_string(),
        ..sample()
    }
}

fn kotlin_gradle_sample() -> ScaffoldSpec {
    ScaffoldSpec {
        build_tool_id: GRADLE.to_string(),
        language_id: KOTLIN.to_string(),
        ..sample()
    }
}

#[test]
fn kotlin_maven_scaffold_puts_sources_under_src_main_kotlin_as_a_kt_file() {
    let files = files_for(&kotlin_maven_sample());
    let paths: Vec<&Path> = files.iter().map(|(p, _)| p.as_path()).collect();
    assert_eq!(
        paths,
        vec![
            Path::new("pom.xml"),
            Path::new("src/main/kotlin/com/example/app/Main.kt"),
            Path::new(".gitignore"),
        ]
    );
}

#[test]
fn kotlin_main_declares_the_package_and_a_top_level_main() {
    let files = files_for(&kotlin_maven_sample());
    let (_, main) = &files[1];
    assert!(main.starts_with("package com.example.app\n\n"), "{main}");
    assert!(main.contains("fun main() {"), "{main}");
    assert!(main.contains("println(\"Hello, world!\")"), "{main}");
    // No wrapper class — the whole point of the idiomatic Kotlin entry
    // point over a Java-style `class Main`.
    assert!(!main.contains("class Main"), "{main}");
}

#[test]
fn kotlin_maven_pom_wires_the_kotlin_plugin_stdlib_and_source_root() {
    let files = files_for(&kotlin_maven_sample());
    let (_, pom) = &files[0];
    assert!(pom.contains("<artifactId>kotlin-maven-plugin</artifactId>"), "{pom}");
    assert!(pom.contains("<artifactId>kotlin-stdlib</artifactId>"), "{pom}");
    assert!(
        pom.contains("<sourceDirectory>src/main/kotlin</sourceDirectory>"),
        "{pom}"
    );
    assert!(
        pom.contains(&format!("<kotlin.version>{KOTLIN_VERSION}</kotlin.version>")),
        "{pom}"
    );
}

#[test]
fn kotlin_maven_pom_release_reads_back_through_java_release_detect() {
    // Same regression guard as the Java pom: a scaffolded Kotlin pom
    // still has to report the wizard's promised level to this project's
    // own detector.
    let spec = ScaffoldSpec {
        runtime_version: Some(8),
        ..kotlin_maven_sample()
    };
    let (_, pom) = &files_for(&spec)[0];
    assert_eq!(
        crate::java_release::release_from_pom(pom).map(|(major, _)| major),
        Some(8)
    );
}

#[test]
fn kotlin_maven_jvm_target_uses_the_legacy_1_8_spelling_for_java_8() {
    let spec = ScaffoldSpec {
        runtime_version: Some(8),
        ..kotlin_maven_sample()
    };
    let (_, pom) = &files_for(&spec)[0];
    assert!(pom.contains("<jvmTarget>1.8</jvmTarget>"), "{pom}");

    let spec = ScaffoldSpec {
        runtime_version: Some(17),
        ..kotlin_maven_sample()
    };
    let (_, pom) = &files_for(&spec)[0];
    assert!(pom.contains("<jvmTarget>17</jvmTarget>"), "{pom}");
}

#[test]
fn kotlin_gradle_scaffold_returns_the_expected_paths() {
    let files = files_for(&kotlin_gradle_sample());
    let paths: Vec<&Path> = files.iter().map(|(p, _)| p.as_path()).collect();
    assert_eq!(
        paths,
        vec![
            Path::new("settings.gradle.kts"),
            Path::new("build.gradle.kts"),
            Path::new("src/main/kotlin/com/example/app/Main.kt"),
            Path::new(".gitignore"),
        ]
    );
}

#[test]
fn kotlin_gradle_build_wires_the_jvm_plugin_and_the_kt_main_class() {
    let files = files_for(&kotlin_gradle_sample());
    let (_, build) = &files[1];
    assert!(
        build.contains(&format!(r#"kotlin("jvm") version "{KOTLIN_VERSION}""#)),
        "{build}"
    );
    // A top-level `fun main` compiles to `<package>.MainKt`, not `Main`.
    assert!(build.contains(r#"mainClass = "com.example.app.MainKt""#), "{build}");
}

#[test]
fn kotlin_gradle_build_release_reads_back_through_java_release_detect() {
    // `kotlin { jvmToolchain(N) }` is exactly the form
    // `release_from_gradle` ranks highest, so a Kotlin Gradle project
    // reads back at the promised release just like the Java one.
    let spec = ScaffoldSpec {
        runtime_version: Some(21),
        ..kotlin_gradle_sample()
    };
    let (_, build) = &files_for(&spec)[1];
    assert_eq!(
        crate::java_release::release_from_gradle(build).map(|(major, _)| major),
        Some(21)
    );
}

#[test]
fn kotlin_single_segment_group_id_still_produces_a_valid_package_path_and_main_class() {
    let spec = ScaffoldSpec {
        namespace: "app".to_string(),
        ..kotlin_gradle_sample()
    };
    let files = files_for(&spec);
    assert_eq!(files[2].0, Path::new("src/main/kotlin/app/Main.kt"));
    assert!(files[2].1.starts_with("package app\n\n"));
    assert!(files[1].1.contains(r#"mainClass = "app.MainKt""#));
}
