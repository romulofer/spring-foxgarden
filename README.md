# spring-foxgarden

Java, Kotlin and Spring support for [FoxGarden](../foxgarden), as an
extension rather than as part of the editor.

FoxGarden's core is language-agnostic (Track 24 in its `PLAN.md`; the
seams are in place, the last JVM specifics are still moving over). Everything that knows what Java, Kotlin, Maven, Gradle, a JDK
or Spring *is* belongs on this side of the seam; the editor keeps the
generic machinery — a text buffer, a tree-sitter parse, an LSP client, a
process runner — and learns about the JVM only through what this crate
contributes.

## The seam

One dependency, `fg-extension`, and nothing else of FoxGarden's. No
`fg-core`, no `syntax`, no app. That is the boundary, and it is enforced
by the compiler: a shortcut into the editor's internals is a build
failure here, not a review comment.

```rust
use fg_extension::{Contributions, Extension};

impl Extension for SpringExtension {
    fn manifest(&self) -> ExtensionManifest { /* id, version, schema_version */ }
    fn contributions(&self) -> Contributions { /* languages, grammars, servers, build tools, analyzers, ... */ }
    // plus defaulted per-file methods: http_routes, doc_stub, import_edit, run_analyzer, ...
}
```

## What it contributes today

| Contribution | Detail |
|---|---|
| Languages | `java` (`.java`), `kotlin` (`.kt`) |
| Grammars | `tree-sitter-java`, `tree-sitter-kotlin-ng`, both compiled in, with their highlight queries (`queries/`) and node-kind vocabularies |
| Language servers | jdt.ls and `kotlin-language-server`: descriptions, `initializationOptions`, JDK discovery, launcher resolution |
| Build tools | Maven and Gradle: detection, build/test/coverage commands, classpaths, compiler-output parsing, JUnit and JaCoCo report reading |
| Scaffolds | New Maven/Gradle projects in Java or Kotlin |
| Run and debug | Run markers (`main` entry points), debug support for Java |
| Type model | Enclosing type, supertype, members, receiver resolution (dot-completion), and code generation: accessors, constructor, `toString`, `equals`/`hashCode`, overrides |
| Imports | Spring annotation candidates after `@` and the `import` edit that makes one resolve |
| Doc comments | Javadoc and KDoc skeletons (`@param`, `@return`, `@throws`, type parameters) |
| Editing aids | Live templates and reserved words for both languages; new-file templates (class plus `package` line) |
| Spring | HTTP route map (`@GetMapping` family), `application.*` configuration keys from the dependency jars' metadata |
| Static analysis | Checkstyle, PMD and SpotBugs as *analyzers*: pinned, checksummed downloads and report parsers |

The queries are forked from each grammar crate's own bundled
`highlights.scm`; their headers record what was changed and why. They live
here, with the grammar they were written against and pinned next to it in
`Cargo.toml`, because a query is meaningless apart from its grammar.

Each of these is a `Contributions` field or an `Extension` method; the
editor consumes them without knowing a Java concept by name. One module per
concern under `crates/spring/src` (`maven.rs`, `analyzers/`, `doc_stub.rs`,
`snippets.rs`, ...), each with a sibling `*_test.rs`.

## What is not here yet

A tail of JVM specifics still lives in the editor's app crate and is moving
across: the language-server installers (and their vendored archives), the
Java-specific parts of the LSP/DAP protocol handling, the JDK registry
panels, and Spring configuration completion.

## How it is loaded

Compiled in, for now. FoxGarden's `crates/languages` names this crate as a
path dependency and registers it at startup — that is Phase A of the track
("open the seams, JVM still compiled in"), which is done. Phase B replaces
that edge with a manifest, a permission model and a real loader, and nothing
here assumes which of the two is doing the loading.

Consequently the two checkouts have to sit side by side:

```
cross_platform_apps/
├── foxgarden/
└── spring-foxgarden/
```

## Build

```
cargo test --workspace
cargo clippy --workspace --all-targets
```

Changes to the seam (`fg-extension`) live in the `foxgarden` checkout; run
its `cargo test --workspace` too.

## License

MIT — see [LICENSE](LICENSE).
