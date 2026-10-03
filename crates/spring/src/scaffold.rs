//! Generates a brand-new project's starting files — pure text generation,
//! mirroring `gradle.rs::INIT_SCRIPT`'s own "Rust string constant, values
//! substituted in" shape. Nothing here runs `mvn`/`gradle`; the result is
//! exactly the files a real `mvn`/`gradle` on the user's own machine can
//! build, and writing them to disk is the editor's job, not this module's.

use std::path::{Path, PathBuf};

use fg_extension::{LanguageId, ScaffoldContribution, ScaffoldSpec};

use crate::build_tools::{GRADLE, MAVEN};
use crate::{JAVA, KOTLIN};

/// The Java releases the wizard offers — every one jdt.ls names correctly as
/// an execution environment, from the oldest release a real project still
/// targets in practice to the newest LTS.
const JAVA_RELEASES: [u32; 4] = [8, 11, 17, 21];

/// The four project kinds this extension scaffolds: each JVM language on
/// each build tool.
pub fn contributions() -> Vec<ScaffoldContribution> {
    [MAVEN, GRADLE]
        .into_iter()
        .flat_map(|tool| {
            [JAVA, KOTLIN].into_iter().map(move |language| ScaffoldContribution {
                build_tool_id: tool.to_string(),
                language_id: LanguageId::from(language),
                runtime_versions: JAVA_RELEASES.to_vec(),
            })
        })
        .collect()
}

/// The Java release a spec asks for, defaulting to the newest offered when
/// it names none — a scaffolded project always has to state one, since both
/// build files this writes declare it explicitly.
fn java_release(spec: &ScaffoldSpec) -> u32 {
    spec.runtime_version.unwrap_or(21)
}

/// A `group_id` like `com.example.app`, as the package-path components
/// `src/main/java` sits under: `["com", "example", "app"]`. Empty/invalid
/// segments (a stray `..`, a leading/trailing dot) are dropped rather than
/// rejected outright — a scaffolded project should never fail to generate
/// over a typo'd group ID; `mvn`/`javac` will say so clearly enough later
/// if what's left doesn't compile.
fn package_path_components(group_id: &str) -> Vec<&str> {
    group_id.split('.').filter(|segment| !segment.is_empty()).collect()
}

/// The generated `pom.xml`: a bare `<maven.compiler.release>` property is
/// enough — no `maven-compiler-plugin` configuration needed, since the
/// plugin already reads that property itself (the exact mechanism `PLAN.md`
/// Track 29 Phase 0 live-verified jdt.ls also honors with zero client-side
/// help).
fn pom_xml(spec: &ScaffoldSpec) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>

  <groupId>{group_id}</groupId>
  <artifactId>{artifact_id}</artifactId>
  <version>1.0-SNAPSHOT</version>
  <packaging>jar</packaging>

  <properties>
    <maven.compiler.release>{java_release}</maven.compiler.release>
    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
  </properties>
</project>
"#,
        group_id = spec.namespace,
        artifact_id = spec.name,
        java_release = java_release(spec),
    )
}

/// The generated `src/main/java/<package path>/Main.java` — a real entry
/// point (unlike `boilerplate::generate`'s own empty-class skeleton for a
/// file added to an *existing* project), so `mvn -q compile`/`exec:java`
/// has something to actually build and run.
fn main_java(spec: &ScaffoldSpec) -> String {
    let package = package_path_components(&spec.namespace).join(".");
    let mut out = String::new();
    if !package.is_empty() {
        out.push_str(&format!("package {package};\n\n"));
    }
    out.push_str("public class Main {\n    public static void main(String[] args) {\n        System.out.println(\"Hello, world!\");\n    }\n}\n");
    out
}

const GITIGNORE: &str = "target/\n*.class\n.idea/\n*.iml\n";
const GRADLE_GITIGNORE: &str = "build/\n.gradle/\n*.class\n.idea/\n*.iml\n";

/// The Kotlin release pinned across every generated Kotlin build file — a
/// real, resolvable Maven Central coordinate for both `kotlin-stdlib` and
/// the `kotlin-maven-plugin`/`kotlin("jvm")` Gradle plugin, so a scaffolded
/// project builds without the user having to pick a version.
const KOTLIN_VERSION: &str = "2.0.21";

/// Kotlin's `jvmTarget` (the Maven plugin's own accepted spelling): `"1.8"`
/// for Java 8, the bare number otherwise. Unlike Gradle's `jvmToolchain`,
/// which takes a plain integer, `kotlin-maven-plugin`'s `<jvmTarget>` still
/// wants the legacy `1.8` form for 8 and rejects `8`.
fn kotlin_jvm_target(java_release: u32) -> String {
    if java_release == 8 {
        "1.8".to_string()
    } else {
        java_release.to_string()
    }
}

/// The generated `pom.xml` for a Kotlin project: `kotlin-stdlib` on the
/// classpath, `src/main/kotlin` as the source root, and the
/// `kotlin-maven-plugin` bound to `compile` (its own documented minimal
/// setup). `<maven.compiler.release>` is kept too — harmless for a
/// Kotlin build, and it keeps `java_release::release_from_pom` reading the
/// wizard's promised level back the same way it does for a Java project.
fn kotlin_pom_xml(spec: &ScaffoldSpec) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 http://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>

  <groupId>{group_id}</groupId>
  <artifactId>{artifact_id}</artifactId>
  <version>1.0-SNAPSHOT</version>
  <packaging>jar</packaging>

  <properties>
    <kotlin.version>{kotlin_version}</kotlin.version>
    <maven.compiler.release>{java_release}</maven.compiler.release>
    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
  </properties>

  <dependencies>
    <dependency>
      <groupId>org.jetbrains.kotlin</groupId>
      <artifactId>kotlin-stdlib</artifactId>
      <version>${{kotlin.version}}</version>
    </dependency>
  </dependencies>

  <build>
    <sourceDirectory>src/main/kotlin</sourceDirectory>
    <plugins>
      <plugin>
        <groupId>org.jetbrains.kotlin</groupId>
        <artifactId>kotlin-maven-plugin</artifactId>
        <version>${{kotlin.version}}</version>
        <executions>
          <execution>
            <id>compile</id>
            <phase>compile</phase>
            <goals>
              <goal>compile</goal>
            </goals>
          </execution>
        </executions>
        <configuration>
          <jvmTarget>{jvm_target}</jvmTarget>
        </configuration>
      </plugin>
    </plugins>
  </build>
</project>
"#,
        group_id = spec.namespace,
        artifact_id = spec.name,
        kotlin_version = KOTLIN_VERSION,
        java_release = java_release(spec),
        jvm_target = kotlin_jvm_target(java_release(spec)),
    )
}

/// The generated `src/main/kotlin/<package path>/Main.kt` — a top-level
/// `fun main()`, the idiomatic Kotlin entry point (no wrapper class), which
/// the compiler exposes as the JVM class `<package>.MainKt` (see
/// `kotlin_build_gradle_kts`'s own `mainClass`).
fn main_kotlin(spec: &ScaffoldSpec) -> String {
    let package = package_path_components(&spec.namespace).join(".");
    let mut out = String::new();
    if !package.is_empty() {
        out.push_str(&format!("package {package}\n\n"));
    }
    out.push_str("fun main() {\n    println(\"Hello, world!\")\n}\n");
    out
}

/// `build.gradle.kts` for a Kotlin project: the `kotlin("jvm")` plugin
/// (which pulls `kotlin-stdlib` in on its own, so no explicit dependency is
/// needed) plus `application`, and `kotlin { jvmToolchain(N) }` — the exact
/// form `java_release::release_from_gradle` ranks highest as
/// `kotlin.jvmToolchain`, so a scaffolded Kotlin project reads back at the
/// release the wizard promised, same guarantee the Java path has. The same
/// no-wrapper stance as `build_gradle_kts` applies (`PLAN.md` Track 29
/// Phase 4).
fn kotlin_build_gradle_kts(spec: &ScaffoldSpec) -> String {
    let package = package_path_components(&spec.namespace).join(".");
    let main_class = if package.is_empty() {
        "MainKt".to_string()
    } else {
        format!("{package}.MainKt")
    };
    format!(
        r#"plugins {{
    kotlin("jvm") version "{kotlin_version}"
    application
}}

kotlin {{
    jvmToolchain({java_release})
}}

application {{
    mainClass = "{main_class}"
}}

group = "{group_id}"
version = "1.0-SNAPSHOT"

repositories {{
    mavenCentral()
}}
"#,
        kotlin_version = KOTLIN_VERSION,
        java_release = java_release(spec),
        group_id = spec.namespace,
    )
}

/// `settings.gradle.kts`: just the root project name, the same single
/// responsibility a real `gradle init` gives this file.
fn settings_gradle_kts(spec: &ScaffoldSpec) -> String {
    format!("rootProject.name = \"{}\"\n", spec.name)
}

/// `build.gradle.kts`. The toolchain block is deliberately the one form
/// `java_release::release_from_gradle` ranks highest (`java.toolchain.
/// languageVersion`) — a scaffolded project has to read back at the release
/// the wizard promised, the same regression `pom_xml`'s own
/// `pom_xml_release_reads_back_through_java_release_detect_at_the_requested_
/// value` test already guards for Maven. No Gradle wrapper is generated
/// (`PLAN.md` Track 29 Phase 4's own stated non-goal — needs a real network
/// fetch or vendoring, the same trade-off `lsp_manager.rs` already reasons
/// through for jdt.ls) — the wizard's own help text says so explicitly
/// rather than leaving a silently-missing `gradlew`.
fn build_gradle_kts(spec: &ScaffoldSpec) -> String {
    let package = package_path_components(&spec.namespace).join(".");
    let main_class = if package.is_empty() {
        "Main".to_string()
    } else {
        format!("{package}.Main")
    };
    format!(
        r#"plugins {{
    java
    application
}}

java {{
    toolchain {{
        languageVersion = JavaLanguageVersion.of({java_release})
    }}
}}

application {{
    mainClass = "{main_class}"
}}

group = "{group_id}"
version = "1.0-SNAPSHOT"

repositories {{
    mavenCentral()
}}
"#,
        java_release = java_release(spec),
        group_id = spec.namespace,
    )
}

/// The files a new project needs, as `(path relative to the project root,
/// content)` pairs — pure generation, no filesystem access, so it's cheap
/// to test exactly (`write_scaffold` is the only function that touches
/// disk).
pub fn scaffold_files(spec: &ScaffoldSpec) -> Option<Vec<(PathBuf, String)>> {
    let package_dir: PathBuf = package_path_components(&spec.namespace).into_iter().collect();
    // Source root and entry-point file follow the language, the same
    // `src/main/<lang>` convention Maven and Gradle both use.
    let (src_root, main_file) = match spec.language_id.as_str() {
        JAVA => ("src/main/java", "Main.java"),
        KOTLIN => ("src/main/kotlin", "Main.kt"),
        _ => return None,
    };
    let main_path = Path::new(src_root).join(package_dir).join(main_file);
    let main_source = match spec.language_id.as_str() {
        JAVA => main_java(spec),
        _ => main_kotlin(spec),
    };
    let files = match (spec.build_tool_id.as_str(), spec.language_id.as_str()) {
        (MAVEN, JAVA) => vec![
            (PathBuf::from("pom.xml"), pom_xml(spec)),
            (main_path, main_source),
            (PathBuf::from(".gitignore"), GITIGNORE.to_string()),
        ],
        (MAVEN, KOTLIN) => vec![
            (PathBuf::from("pom.xml"), kotlin_pom_xml(spec)),
            (main_path, main_source),
            (PathBuf::from(".gitignore"), GITIGNORE.to_string()),
        ],
        (GRADLE, JAVA) => vec![
            (PathBuf::from("settings.gradle.kts"), settings_gradle_kts(spec)),
            (PathBuf::from("build.gradle.kts"), build_gradle_kts(spec)),
            (main_path, main_source),
            (PathBuf::from(".gitignore"), GRADLE_GITIGNORE.to_string()),
        ],
        (GRADLE, KOTLIN) => vec![
            (PathBuf::from("settings.gradle.kts"), settings_gradle_kts(spec)),
            (PathBuf::from("build.gradle.kts"), kotlin_build_gradle_kts(spec)),
            (main_path, main_source),
            (PathBuf::from(".gitignore"), GRADLE_GITIGNORE.to_string()),
        ],
        _ => return None,
    };
    Some(files)
}

#[cfg(test)]
#[path = "scaffold_test.rs"]
mod scaffold_test;
