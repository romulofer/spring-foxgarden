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
//! grammars and their highlight queries. Track 24's later phases bring the
//! rest across (language servers in Phase 4; build, run, debug, profile and
//! test reporting in Phase 5; the Spring panels and editor behaviors in
//! Phase 6), each one arriving as more `Contributions` rather than as more
//! reach into the core.
//!
//! **Still compiled into the binary.** Phase A of the track keeps every
//! extension linked in — FoxGarden's own `fg-languages` names this crate as
//! a path dependency and registers it at startup. Phase B is what replaces
//! that edge with a manifest and a loader, and nothing in this crate's API
//! assumes which of the two it is being loaded by.

use fg_extension::{
    Contributions, Extension, ExtensionManifest, GrammarContribution, GrammarSource,
    LanguageContribution, LanguageId, CURRENT_SCHEMA_VERSION,
};

/// The language ids this extension contributes. Public because a
/// contribution from elsewhere (a language server serving Java, say) has to
/// name them, and an id typed as a string literal in two places is an id
/// that will eventually be typed wrong in one of them.
pub const JAVA: &str = "java";
pub const KOTLIN: &str = "kotlin";

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
                },
            ],
            ..Default::default()
        }
    }
}

fn language(id: &str, display_name: &str, extension: &str) -> LanguageContribution {
    LanguageContribution {
        id: LanguageId::from(id),
        display_name: display_name.to_string(),
        file_extensions: vec![extension.to_string()],
        filename_patterns: Vec::new(),
    }
}

#[cfg(test)]
#[path = "lib_test.rs"]
mod lib_test;
