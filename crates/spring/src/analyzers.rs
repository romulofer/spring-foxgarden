//! The JVM static-analysis tools: Checkstyle, PMD and SpotBugs.
//!
//! Each is a command-line tool the user points the editor at, or has it
//! download. This module declares them (`contributions`) and runs them
//! (`run`); each tool's module owns the shape of its own report, since the
//! three formats aren't related — attribute-only vs. text-content messages,
//! a point column vs. a real begin/end range vs. no column at all, and
//! SpotBugs analyzing compiled bytecode rather than source text.
//!
//! Every version, URL and archive-layout claim below was verified against a
//! real download, not assumed. The pinned versions in particular: Checkstyle's
//! newest major line (13.x) requires a newer JDK than many real installs
//! have, and running it produced a real `UnsupportedClassVersionError`
//! against a genuine Java 17 JVM. 10.26.1 is the newest Checkstyle release
//! confirmed (by actually running it) to still work under Java 17. PMD
//! 7.26.0 and SpotBugs 4.10.3 were verified under the same JVM and are
//! pinned too, so "the version this installs" is reproducible for every tool.

use fg_extension::{
    AnalyzerContribution, AnalyzerFinding, AnalyzerInput, AnalyzerInstall, AnalyzerPackaging, AnalyzerRun, LatestRelease,
};

mod checkstyle;
mod pmd;
mod report;
mod spotbugs;

pub const CHECKSTYLE: &str = "checkstyle";
pub const PMD: &str = "pmd";
pub const SPOTBUGS: &str = "spotbugs";

pub fn contributions() -> Vec<AnalyzerContribution> {
    vec![
        AnalyzerContribution {
            id: CHECKSTYLE.to_string(),
            display_name: "Checkstyle".to_string(),
            input: AnalyzerInput::Sources,
            // Checkstyle has no usable default ruleset of its own: every real
            // run needs an explicit `-c`.
            config_label: Some("Config (-c)".to_string()),
            install: Some(AnalyzerInstall {
                version: "10.26.1".to_string(),
                download_url: "https://github.com/checkstyle/checkstyle/releases/download/checkstyle-10.26.1/\
                               checkstyle-10.26.1-all.jar"
                    .to_string(),
                sha256: "e41c24433723ba310a30e41da4f449c105ad47cab2ae9e6be5e06606a647dbca".to_string(),
                // Checkstyle ships a single self-contained jar, no launcher.
                packaging: AnalyzerPackaging::File {
                    name: "checkstyle-10.26.1-all.jar".to_string(),
                },
                default_config: Some("/sun_checks.xml".to_string()),
                latest_release: Some(LatestRelease {
                    api_url: "https://api.github.com/repos/checkstyle/checkstyle/releases/latest".to_string(),
                    tag_prefix: "checkstyle-".to_string(),
                }),
            }),
        },
        AnalyzerContribution {
            id: PMD.to_string(),
            display_name: "PMD".to_string(),
            input: AnalyzerInput::Sources,
            // PMD's `-R` — one ruleset path, or several comma-separated (its
            // own `-R=<rulesets>[,<rulesets>...]` shape, forwarded verbatim).
            // Required the same way Checkstyle's config is.
            config_label: Some("Ruleset (-R)".to_string()),
            install: Some(AnalyzerInstall {
                version: "7.26.0".to_string(),
                download_url: "https://github.com/pmd/pmd/releases/download/pmd_releases/7.26.0/\
                               pmd-dist-7.26.0-bin.zip"
                    .to_string(),
                sha256: "9f55cb7ff0e9f9a66dd2f005eaa370e84c8a4cd971b134aa14a930c4a283ebc9".to_string(),
                packaging: AnalyzerPackaging::Zip {
                    dir_hint: "pmd".to_string(),
                    launcher: if cfg!(windows) { "bin/pmd.bat" } else { "bin/pmd" }.to_string(),
                },
                default_config: Some("rulesets/java/quickstart.xml".to_string()),
                latest_release: Some(LatestRelease {
                    api_url: "https://api.github.com/repos/pmd/pmd/releases/latest".to_string(),
                    tag_prefix: "pmd_releases/".to_string(),
                }),
            }),
        },
        AnalyzerContribution {
            id: SPOTBUGS.to_string(),
            display_name: "SpotBugs".to_string(),
            // Analyzes bytecode, so the project has to be built first.
            input: AnalyzerInput::BuildOutput,
            config_label: None,
            install: Some(AnalyzerInstall {
                version: "4.10.3".to_string(),
                download_url: "https://github.com/spotbugs/spotbugs/releases/download/4.10.3/spotbugs-4.10.3.zip"
                    .to_string(),
                sha256: "e814ee5bf9665412658c4d684e45eae3cf993148a71bc8bc93fb343e92288151".to_string(),
                packaging: AnalyzerPackaging::Zip {
                    dir_hint: "spotbugs".to_string(),
                    // The Windows launcher isn't `fb` with an extension: it is
                    // a *differently named* script, `spotbugs.bat` (there is no
                    // `fb.bat` in the archive at all).
                    launcher: if cfg!(windows) { "bin/spotbugs.bat" } else { "bin/fb" }.to_string(),
                },
                default_config: None,
                latest_release: Some(LatestRelease {
                    api_url: "https://api.github.com/repos/spotbugs/spotbugs/releases/latest".to_string(),
                    tag_prefix: String::new(),
                }),
            }),
        },
    ]
}

/// Runs analyzer `analyzer_id`. `Err` for one this module doesn't know.
pub fn run(analyzer_id: &str, run: &AnalyzerRun) -> Result<Vec<AnalyzerFinding>, String> {
    match analyzer_id {
        CHECKSTYLE => checkstyle::analyze(run),
        PMD => pmd::analyze(run),
        SPOTBUGS => spotbugs::analyze(run),
        other => Err(format!("unknown analyzer {other}")),
    }
}

#[cfg(test)]
#[path = "analyzers_test.rs"]
mod analyzers_test;
