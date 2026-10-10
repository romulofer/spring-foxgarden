//! The Spring/JVM add-on for FoxGarden.
//!
//! FoxGarden's core is being made language-agnostic (`PLAN.md` Track 24 in
//! that repository); everything that knows what Java, Kotlin, Maven,
//! Gradle, a JDK or Spring *is* belongs here instead. This crate reaches
//! the editor through exactly one seam — `fg_extension::Extension` — and
//! depends on nothing else of FoxGarden's, so the boundary is enforced by
//! the compiler rather than by discipline.
//!
//! **What is here today**: the two JVM languages, their tree-sitter
//! grammars and their highlight queries, plus the two language servers and
//! the JDK discovery logic they need (Phase 4), plus Maven and Gradle
//! themselves as contributed build tools — detection, build/test/coverage
//! commands, classpath resolution, compiler-output parsing and test/coverage
//! report reading (Phase 5). Track 24's later phases bring the rest across
//! (the Spring panels and editor behaviors in Phase 6), each one arriving as
//! more `Contributions` rather than as more reach into the core.
//!
//! **Still compiled into the binary.** Phase A of the track keeps every
//! extension linked in — FoxGarden's own `fg-languages` names this crate as
//! a path dependency and registers it at startup. Phase B is what replaces
//! that edge with a manifest and a loader, and nothing in this crate's API
//! assumes which of the two it is being loaded by.

use std::path::{Path, PathBuf};
use std::process::Command;

use fg_extension::{
    AnalyzerFinding, AnalyzerRun, BuildProblem, BuildTask, BuildToolHandle, CodeGeneration, CommandSpec, ConfigProperty, Contributions, CoverageReport,
    Extension, ExtensionManifest, GrammarContribution, GrammarSource, HttpRoute, ImportCandidate, ImportEdit, JdkRuntime, LanguageContribution,
    LanguageId, LanguageServerContribution, MemberView, NodeKinds, ProjectRelease, ReceiverType, ResolvedServerStart,
    RunSpec, RunTarget, ScaffoldSpec, ServerStartContext, TestCase, TestFailureLocation, TypeDeclaration, TypeFields,
    TypeMember, CURRENT_SCHEMA_VERSION,
};

mod analyzers;
mod annotations;
mod boilerplate;
pub mod build_tools;
mod code_model;
mod codegen;
pub mod config_metadata;
pub mod coverage;
mod fields;
pub mod gradle;
pub mod http_routes;
mod identifier_type;
mod imports;
pub mod java_release;
mod kotlin_members;
pub mod main_entry;
pub mod scaffold;
pub mod maven;
mod methods;
pub mod run;
pub mod test_report;
mod xml;

pub use build_tools::{GRADLE, MAVEN};

/// The language ids this extension contributes. Public because a
/// contribution from elsewhere (a language server serving Java, say) has to
/// name them, and an id typed as a string literal in two places is an id
/// that will eventually be typed wrong in one of them.
pub const JAVA: &str = "java";
pub const KOTLIN: &str = "kotlin";

/// Server ids this extension registers. Public for the same reason as the
/// language ids — callers that need to look up a server by id should use
/// these rather than string literals.
pub const JDTLS: &str = "jdtls";
pub const KOTLIN_LANGUAGE_SERVER: &str = "kotlin-language-server";

/// The JVM jdt.ls itself requires to *run*.
const JDTLS_MINIMUM_JDK: u32 = 21;

/// `com.microsoft.java.debug.plugin` bytes, vendored from the FoxGarden tree
/// via the path dependency. Included here so the spring extension is the only
/// thing that knows this jar exists.
const JAVA_DEBUG_PLUGIN_JAR: &[u8] =
    include_bytes!("../../../../foxgarden/vendor/lsp-servers/java-debug-plugin-0.53.2.jar");
const JAVA_DEBUG_PLUGIN_VERSION: &str = "0.53.2";

/// Java, Kotlin and (eventually) everything Spring.
pub struct SpringExtension;

impl Extension for SpringExtension {
    fn manifest(&self) -> ExtensionManifest {
        ExtensionManifest {
            id: "spring".to_string(),
            name: "Java, Kotlin and Spring".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            schema_version: CURRENT_SCHEMA_VERSION,
        }
    }

    fn contributions(&self) -> Contributions {
        Contributions {
            languages: vec![language(JAVA, "Java", "java"), language(KOTLIN, "Kotlin", "kt")],
            grammars: vec![
                GrammarContribution {
                    language_id: JAVA.to_string(),
                    source: GrammarSource::Builtin(tree_sitter_java::LANGUAGE),
                    highlight_query: Some(include_str!("../queries/highlights_java.scm").to_string()),
                    node_kinds: java_node_kinds(),
                },
                GrammarContribution {
                    language_id: KOTLIN.to_string(),
                    source: GrammarSource::Builtin(tree_sitter_kotlin_ng::LANGUAGE),
                    // Both queries are forked from their grammar crate's own
                    // bundled `highlights.scm` and travel with this crate,
                    // not with the editor: a query is written against one
                    // grammar's node names, and the grammar is pinned right
                    // here in `Cargo.toml`. Their headers record what was
                    // changed and why.
                    highlight_query: Some(include_str!("../queries/highlights_kotlin.scm").to_string()),
                    node_kinds: kotlin_node_kinds(),
                },
            ],
            analyzers: analyzers::contributions(),
            language_servers: vec![
                LanguageServerContribution {
                    id: JDTLS.to_string(),
                    display_name: "Eclipse JDT Language Server".to_string(),
                    language_ids: vec![JAVA.to_string()],
                    binary_name: String::new(),
                    args: Vec::new(),
                    initialization_options: None,
                },
                LanguageServerContribution {
                    id: KOTLIN_LANGUAGE_SERVER.to_string(),
                    display_name: "Kotlin Language Server".to_string(),
                    language_ids: vec![KOTLIN.to_string()],
                    binary_name: String::new(),
                    args: Vec::new(),
                    initialization_options: None,
                },
            ],
            build_tools: build_tools::contributions(),
            scaffolds: scaffold::contributions(),
            http_route_languages: vec![JAVA.to_string(), KOTLIN.to_string()],
        }
    }

    fn jdk_runtimes(&self) -> Vec<JdkRuntime> {
        jdk_runtimes_snapshot()
    }

    fn resolve_server_start(&self, server_id: &str, ctx: &ServerStartContext) -> Option<Result<ResolvedServerStart, String>> {
        match server_id {
            JDTLS => Some(resolve_jdtls(ctx)),
            KOTLIN_LANGUAGE_SERVER => Some(resolve_kotlin_ls(ctx)),
            _ => None,
        }
    }

    fn build_command(&self, tool_id: &str, project_root: &Path, task: BuildTask) -> Option<CommandSpec> {
        build_tools::command(tool_id, project_root, task)
    }

    fn run_command(&self, tool_id: &str, project_root: &Path, run: &RunSpec) -> Option<Result<CommandSpec, String>> {
        run::command(tool_id, project_root, run)
    }

    fn classes_dir(&self, tool_id: &str, project_root: &Path) -> Option<PathBuf> {
        build_tools::classes_dir(tool_id, project_root)
    }

    fn runtime_classpath(&self, tool_id: &str, project_root: &Path) -> Option<Result<Vec<PathBuf>, String>> {
        run::runtime_classpath(tool_id, project_root)
    }

    fn analysis_classpath(&self, tool_id: &str, project_root: &Path) -> Vec<PathBuf> {
        run::analysis_classpath(tool_id, project_root)
    }

    fn parse_build_output_line(&self, _tool_id: &str, line: &str) -> Option<BuildProblem> {
        build_tools::parse_output_line(line)
    }

    fn coverage_results(
        &self,
        tool_id: &str,
        project_root: &Path,
    ) -> Option<Result<CoverageReport, String>> {
        // Coverage exists for Maven only — JaCoCo invoked as a bare plugin
        // goal, which Gradle has no equivalent of without editing the
        // project's own build script.
        (tool_id == MAVEN).then(|| coverage::results(project_root))
    }

    fn test_results(&self, tool_id: &str, project_root: &Path) -> Vec<TestCase> {
        test_report::scan_test_reports(tool_id, project_root)
    }

    fn test_failure_location(
        &self,
        _tool_id: &str,
        project_root: &Path,
        case: &TestCase,
    ) -> Option<TestFailureLocation> {
        test_report::failure_location(project_root, case)
    }

    fn scaffold_files(&self, spec: &ScaffoldSpec) -> Option<Vec<(PathBuf, String)>> {
        scaffold::scaffold_files(spec)
    }

    fn run_targets(
        &self,
        language_id: &str,
        tree: &tree_sitter::Tree,
        source: &str,
        file_stem: &str,
    ) -> Vec<RunTarget> {
        main_entry::main_entries(tree, source, language_id, file_stem)
    }

    fn http_routes(&self, language_id: &str, tree: &tree_sitter::Tree, source: &str) -> Vec<HttpRoute> {
        http_routes::http_routes(tree, source, language_id)
    }

    fn enclosing_type(
        &self,
        language_id: &str,
        tree: &tree_sitter::Tree,
        source: &str,
        byte: usize,
    ) -> Option<TypeDeclaration> {
        code_model::enclosing_type(language_id, tree, source, byte)
    }

    fn supertype(&self, language_id: &str, tree: &tree_sitter::Tree, source: &str, type_name: &str) -> Option<String> {
        code_model::supertype(language_id, tree, source, type_name)
    }

    fn type_members(
        &self,
        language_id: &str,
        tree: &tree_sitter::Tree,
        source: &str,
        type_name: &str,
        view: MemberView,
    ) -> Vec<TypeMember> {
        code_model::type_members(language_id, tree, source, type_name, view)
    }

    fn receiver_type(
        &self,
        language_id: &str,
        tree: &tree_sitter::Tree,
        source: &str,
        byte: usize,
        receiver: &str,
    ) -> Option<ReceiverType> {
        code_model::receiver_type(language_id, tree, source, byte, receiver)
    }

    fn generates_code(&self, language_id: &str) -> bool {
        language_id == JAVA
    }

    fn types_with_fields(&self, language_id: &str, tree: &tree_sitter::Tree, source: &str) -> Vec<TypeFields> {
        code_model::types_with_fields(language_id, tree, source)
    }

    fn generate_code(&self, language_id: &str, request: CodeGeneration<'_>, indent_unit: &str) -> Option<String> {
        self.generates_code(language_id).then(|| codegen::generate(request, indent_unit))
    }

    fn run_analyzer(&self, analyzer_id: &str, run: &AnalyzerRun) -> Result<Vec<AnalyzerFinding>, String> {
        analyzers::run(analyzer_id, run)
    }

    fn annotation_candidates(&self, language_id: &str) -> Vec<ImportCandidate> {
        annotations::candidates(language_id)
    }

    fn import_edit(
        &self,
        language_id: &str,
        tree: &tree_sitter::Tree,
        source: &str,
        qualified_name: &str,
    ) -> Option<ImportEdit> {
        imports::import_edit(tree, source, language_id, qualified_name)
    }

    fn new_file_template(&self, language_id: &str, project_root: &Path, file_path: &Path) -> Option<String> {
        boilerplate::generate(language_id, project_root, file_path)
    }

    fn project_release(&self, project_root: &Path) -> Option<ProjectRelease> {
        java_release::detect(project_root)
    }

    fn config_properties(&self, project_root: &Path, build_tool: &BuildToolHandle) -> Vec<ConfigProperty> {
        config_metadata::properties_for_project(project_root, build_tool)
    }
}

/// Java's node vocabulary. Declarations for sticky scroll, bodies and block
/// comments for folding, `import_declaration` for folding an import block as
/// a unit — every name taken from `tree-sitter-java`'s own parse output.
fn java_node_kinds() -> NodeKinds {
    NodeKinds {
        scopes: names(&[
            "class_declaration",
            "interface_declaration",
            "enum_declaration",
            "record_declaration",
            "annotation_type_declaration",
            "method_declaration",
            "constructor_declaration",
        ]),
        foldable: names(&[
            "class_body",
            "interface_body",
            "enum_body",
            "annotation_type_body",
            "constructor_body",
            "block", // method and control-flow bodies
            "block_comment",
        ]),
        import: Some("import_declaration".to_string()),
    }
}

/// Kotlin's, verified against `tree-sitter-kotlin-ng` 1.1.0's real parse
/// output rather than assumed from the Java grammar: `class_body` covers
/// class/interface/object bodies alike (interface and object declarations
/// reuse `class_declaration`/`class_body`, distinguished only by keyword),
/// `enum_class_body` is the one exception with its own kind, `block` is
/// method *and* control-flow bodies alike (a `function_body` node wraps a
/// `block` at the exact same span, so folding `block` covers both without a
/// second redundant entry), and `block_comment` covers regular comments and
/// KDoc alike.
///
/// No scopes yet: sticky scroll's Kotlin vocabulary still has to be derived
/// from the grammar's own `node-types.json` the same way these were, and a
/// guess copied from Java would pin the wrong lines.
fn kotlin_node_kinds() -> NodeKinds {
    NodeKinds {
        scopes: Vec::new(),
        foldable: names(&["class_body", "enum_class_body", "block", "block_comment"]),
        import: Some("import".to_string()),
    }
}

fn names(kinds: &[&str]) -> Vec<String> {
    kinds.iter().map(|k| (*k).to_string()).collect()
}

fn language(id: &str, display_name: &str, extension: &str) -> LanguageContribution {
    LanguageContribution {
        id: LanguageId::from(id),
        display_name: display_name.to_string(),
        file_extensions: vec![extension.to_string()],
        filename_patterns: Vec::new(),
    }
}

// ──────────────────────────────────────────────────────────────────────────
// jdt.ls startup

fn resolve_jdtls(ctx: &ServerStartContext) -> Result<ResolvedServerStart, String> {
    let java = resolve_jdtls_java(&ctx.java_home)?;
    let debug_bundles = debug_plugin_bundles();
    let init_opts = jdtls_initialization_options(ctx, &debug_bundles);
    let restart_key = jdtls_restart_key(ctx, &debug_bundles);
    Ok(ResolvedServerStart {
        binary: PathBuf::from(ctx.configured_binary.trim()),
        args: vec!["--java-executable".to_string(), java.display().to_string()],
        initialization_options: Some(init_opts),
        restart_key,
    })
}

fn jdtls_initialization_options(ctx: &ServerStartContext, debug_bundles: &[String]) -> String {
    let runtimes: Vec<serde_json::Value> = ctx
        .jdk_runtimes
        .iter()
        .map(|r| {
            serde_json::json!({
                "name": r.name,
                "path": r.path.display().to_string(),
                "default": Some(r.major) == ctx.java_release,
            })
        })
        .collect();
    serde_json::json!({
        "extendedClientCapabilities": {
            "classFileContentsSupport": true,
            "resolveAdditionalTextEditsSupport": true,
        },
        "settings": { "java": { "configuration": { "runtimes": runtimes } } },
        "bundles": debug_bundles,
    })
    .to_string()
}

fn jdtls_restart_key(ctx: &ServerStartContext, debug_bundles: &[String]) -> String {
    let runtimes_key: String = ctx
        .jdk_runtimes
        .iter()
        .map(|r| format!("{}:{}", r.major, r.path.display()))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "{binary}|{java_home}|{java_release:?}|{runtimes_key}|{debug_bundles:?}",
        binary = ctx.configured_binary.trim(),
        java_home = ctx.java_home,
        java_release = ctx.java_release,
    )
}

/// Checks that `java_home` (or auto-detected JVM) satisfies jdt.ls' Java 21
/// minimum and returns the `java` executable path to pass via
/// `--java-executable`.
fn resolve_jdtls_java(java_home: &str) -> Result<PathBuf, String> {
    let java = java_command(java_home);
    let major = detect_major_version_at(&java).map_err(|e| {
        format!("{e} — install a JDK {JDTLS_MINIMUM_JDK}+, or set it in Settings > Language Servers…")
    })?;
    if major < JDTLS_MINIMUM_JDK {
        return Err(format!(
            "jdt.ls needs a JDK {JDTLS_MINIMUM_JDK} or newer to run, but {} is Java {major} — \
             install a newer JDK, or point Settings > Language Servers… at one",
            java.display()
        ));
    }
    Ok(java)
}

fn java_command(java_home: &str) -> PathBuf {
    let home = if java_home.trim().is_empty() {
        std::env::var_os("JAVA_HOME").map(PathBuf::from)
    } else {
        Some(PathBuf::from(java_home.trim()))
    };
    match home {
        Some(home) => home.join("bin").join("java"),
        None => PathBuf::from("java"),
    }
}

fn detect_major_version_at(java: &Path) -> Result<u32, String> {
    let output = Command::new(java)
        .arg("-version")
        .output()
        .map_err(|e| format!("couldn't run {}: {e}", java.display()))?;
    let banner = String::from_utf8_lossy(&output.stderr);
    java_major_version(&banner).ok_or_else(|| {
        format!("couldn't read a version out of `{} -version`", java.display())
    })
}

fn java_major_version(version_output: &str) -> Option<u32> {
    let quoted = version_output.split('"').nth(1)?;
    let mut parts = quoted.split(['.', '_', '-', '+']);
    let first = parts.next()?;
    if first == "1" {
        parts.next()?.parse().ok()
    } else {
        first.parse().ok()
    }
}

fn debug_plugin_bundles() -> Vec<String> {
    match write_debug_plugin_jar() {
        Ok(path) => vec![path.display().to_string()],
        Err(error) => {
            eprintln!("java-debug plugin unavailable, Debug will not work this session: {error}");
            Vec::new()
        }
    }
}

fn write_debug_plugin_jar() -> Result<PathBuf, String> {
    let dir = lsp_cache_dir()?;
    let path = dir.join(format!("java-debug-plugin-{JAVA_DEBUG_PLUGIN_VERSION}.jar"));
    let already_current =
        std::fs::metadata(&path).map(|m| m.len() as usize).ok() == Some(JAVA_DEBUG_PLUGIN_JAR.len());
    if !already_current {
        std::fs::create_dir_all(&dir).map_err(|e| format!("couldn't create {}: {e}", dir.display()))?;
        std::fs::write(&path, JAVA_DEBUG_PLUGIN_JAR)
            .map_err(|e| format!("couldn't write {}: {e}", path.display()))?;
    }
    Ok(path)
}

// ──────────────────────────────────────────────────────────────────────────
// Kotlin Language Server startup

fn resolve_kotlin_ls(ctx: &ServerStartContext) -> Result<ResolvedServerStart, String> {
    let binary = PathBuf::from(ctx.configured_binary.trim());
    ensure_kotlin_stdlib_override_for(&binary);
    Ok(ResolvedServerStart {
        binary,
        args: Vec::new(),
        initialization_options: Some("{}".to_string()),
        restart_key: String::new(),
    })
}

fn ensure_kotlin_stdlib_override_for(binary: &Path) {
    let root = match xdg_config_root() {
        Ok(root) => root,
        Err(e) => {
            eprintln!("kotlin-language-server stdlib override: {e}");
            return;
        }
    };
    if let Err(e) = ensure_kotlin_stdlib_override(binary, &root) {
        eprintln!("kotlin-language-server stdlib override: {e}");
    }
}

fn ensure_kotlin_stdlib_override(binary: &Path, config_root: &Path) -> Result<(), String> {
    let lib_dir = binary
        .parent()
        .and_then(Path::parent)
        .map(|server_dir| server_dir.join("lib"))
        .ok_or_else(|| format!("couldn't find a lib/ directory next to {}", binary.display()))?;

    let jars = kotlin_stdlib_jars(&lib_dir)?;
    if jars.is_empty() {
        return Err(format!("no kotlin-stdlib*.jar found in {}", lib_dir.display()));
    }

    let script_path = kotlin_classpath_override_path(config_root);
    let script = kotlin_classpath_override_script(&jars);
    if std::fs::read_to_string(&script_path).ok().as_deref() == Some(script.as_str()) {
        return Ok(());
    }

    if let Some(parent) = script_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("couldn't create {}: {e}", parent.display()))?;
    }
    std::fs::write(&script_path, &script).map_err(|e| format!("couldn't write {}: {e}", script_path.display()))?;
    ensure_executable(&script_path)?;
    Ok(())
}

fn kotlin_stdlib_jars(lib_dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = std::fs::read_dir(lib_dir).map_err(|e| format!("couldn't read {}: {e}", lib_dir.display()))?;
    let mut jars: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name.starts_with("kotlin-stdlib")
                && name.ends_with(".jar")
                && !name.contains("-common")
                && !name.contains("-sources")
        })
        .collect();
    jars.sort();
    Ok(jars)
}

fn kotlin_classpath_override_path(config_root: &Path) -> PathBuf {
    let name = if cfg!(windows) { "classpath.bat" } else { "classpath" };
    config_root.join("kotlin-language-server").join(name)
}

fn kotlin_classpath_override_script(jars: &[PathBuf]) -> String {
    let separator = if cfg!(windows) { ';' } else { ':' };
    let joined = jars
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(&separator.to_string());
    if cfg!(windows) {
        format!("@echo off\r\necho {joined}\r\n")
    } else {
        format!("#!/bin/sh\necho \"{joined}\"\n")
    }
}

fn xdg_config_root() -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME") {
        return Ok(PathBuf::from(dir));
    }
    directories::UserDirs::new()
        .map(|dirs| dirs.home_dir().join(".config"))
        .ok_or_else(|| "couldn't determine a home directory".to_string())
}

// ──────────────────────────────────────────────────────────────────────────
// JDK discovery — background scan, same logic as `lsp_manager` in the main
// tree. The copy there goes away once the JDK installer panel moves here
// too (Track 24 Phase 6).

static INSTALLED_RUNTIMES: std::sync::OnceLock<std::sync::Mutex<Option<Vec<JdkRuntime>>>> =
    std::sync::OnceLock::new();
static RUNTIME_SCAN_STARTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn runtimes_cache() -> &'static std::sync::Mutex<Option<Vec<JdkRuntime>>> {
    INSTALLED_RUNTIMES.get_or_init(|| std::sync::Mutex::new(None))
}

/// Non-blocking snapshot of the JDK scan result. Empty until the background
/// scan (started here on first call) completes.
fn jdk_runtimes_snapshot() -> Vec<JdkRuntime> {
    if let Some(found) = runtimes_cache().lock().ok().and_then(|c| c.clone()) {
        return found;
    }
    if !RUNTIME_SCAN_STARTED.swap(true, std::sync::atomic::Ordering::SeqCst) {
        std::thread::spawn(|| {
            let found = scan_installed_runtimes();
            if let Ok(mut cached) = runtimes_cache().lock() {
                *cached = Some(found);
            }
        });
    }
    Vec::new()
}

fn scan_installed_runtimes() -> Vec<JdkRuntime> {
    let candidates = jdk_home_candidates(
        std::env::var_os("JAVA_HOME").map(PathBuf::from),
        java_on_path(),
        &jdk_search_roots(),
    );
    let mut runtimes: Vec<JdkRuntime> = Vec::new();
    for home in candidates {
        let Some(major) = java_major_at(&home) else {
            continue;
        };
        if runtimes.iter().any(|r| r.major == major) {
            continue;
        }
        runtimes.push(JdkRuntime {
            major,
            name: execution_environment_name(major),
            path: home,
        });
    }
    runtimes
}

fn execution_environment_name(major: u32) -> String {
    match major {
        0..=5 => "J2SE-1.5".to_string(),
        6..=8 => format!("JavaSE-1.{major}"),
        _ => format!("JavaSE-{major}"),
    }
}

fn java_on_path() -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|dir| dir.join("java"))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| std::fs::canonicalize(candidate).ok())
}

fn jdk_search_roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/usr/lib/jvm"),
        PathBuf::from("/usr/lib64/jvm"),
        PathBuf::from("/opt/java"),
        PathBuf::from("/Library/Java/JavaVirtualMachines"),
    ];
    if let Some(home) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
        roots.push(home.join(".sdkman/candidates/java"));
        roots.push(home.join(".asdf/installs/java"));
        roots.push(home.join(".jdks"));
        roots.push(home.join("Library/Java/JavaVirtualMachines"));
    }
    #[cfg(windows)]
    {
        for program_files in [std::env::var_os("ProgramFiles"), std::env::var_os("ProgramFiles(x86)")]
            .into_iter()
            .flatten()
        {
            let base = PathBuf::from(program_files);
            roots.push(base.join("Java"));
            roots.push(base.join("Eclipse Adoptium"));
            roots.push(base.join("Amazon Corretto"));
            roots.push(base.join("Microsoft"));
            roots.push(base.join("Zulu"));
        }
    }
    roots
}

fn java_home_at(dir: &Path) -> Option<PathBuf> {
    [dir.to_path_buf(), dir.join("Contents").join("Home")]
        .into_iter()
        .find(|home| home.join("bin").join("java").is_file())
}

fn version_hint(name: &str) -> u32 {
    name.split(|c: char| !c.is_ascii_digit())
        .find(|part| !part.is_empty())
        .and_then(|part| part.parse().ok())
        .unwrap_or(0)
}

fn jdk_home_candidates(
    env_home: Option<PathBuf>,
    path_java: Option<PathBuf>,
    roots: &[PathBuf],
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    candidates.extend(env_home);
    candidates.extend(path_java.and_then(|java| java.parent()?.parent().map(Path::to_path_buf)));

    for root in roots {
        let Ok(entries) = std::fs::read_dir(root) else {
            continue;
        };
        let mut installs: Vec<_> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect();
        installs.sort_by(|a, b| {
            let hint =
                |path: &Path| version_hint(&path.file_name().unwrap_or_default().to_string_lossy());
            hint(b).cmp(&hint(a)).then_with(|| b.file_name().cmp(&a.file_name()))
        });
        candidates.extend(installs);
    }

    let mut seen = std::collections::HashSet::new();
    candidates
        .iter()
        .filter_map(|candidate| java_home_at(candidate))
        .filter(|home| seen.insert(std::fs::canonicalize(home).unwrap_or_else(|_| home.clone())))
        .collect()
}

fn java_major_at(home: &Path) -> Option<u32> {
    detect_major_version_at(&home.join("bin").join("java")).ok()
}

// ──────────────────────────────────────────────────────────────────────────
// Shared utilities

fn lsp_cache_dir() -> Result<PathBuf, String> {
    let base = directories::ProjectDirs::from("", "", "foxgarden")
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .or_else(|| directories::BaseDirs::new().map(|dirs| dirs.data_local_dir().join("foxgarden")))
        .ok_or("could not resolve data dir")?;
    Ok(base.join("lsp"))
}

fn ensure_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)
            .map_err(|e| format!("couldn't read {}: {e}", path.display()))?
            .permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(path, perms)
            .map_err(|e| format!("couldn't make {} executable: {e}", path.display()))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "lib_test.rs"]
mod lib_test;
