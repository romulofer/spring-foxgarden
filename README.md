# spring-foxgarden

Java, Kotlin and Spring support for [FoxGarden](../foxgarden), as an
extension rather than as part of the editor.

FoxGarden's core is being made language-agnostic (Track 24 in its
`PLAN.md`). Everything that knows what Java, Kotlin, Maven, Gradle, a JDK
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
    fn contributions(&self) -> Contributions { /* languages, grammars, servers */ }
}
```

## What it contributes today

| Contribution | Detail |
|---|---|
| Languages | `java` (`.java`), `kotlin` (`.kt`) |
| Grammars | `tree-sitter-java`, `tree-sitter-kotlin-ng`, both compiled in |
| Highlight queries | `queries/highlights_java.scm`, `queries/highlights_kotlin.scm` |

The queries are forked from each grammar crate's own bundled
`highlights.scm`; their headers record what was changed and why. They live
here, with the grammar they were written against and pinned next to it in
`Cargo.toml`, because a query is meaningless apart from its grammar.

## What arrives next

Track 24's remaining phases move the rest across, each as more
`Contributions` rather than as more reach into the core:

- **Phase 4** — jdt.ls and `kotlin-language-server`: the server
  descriptions, their `initializationOptions`, JDK discovery and launcher
  resolution.
- **Phase 5** — Maven and Gradle model extraction, classpath resolution,
  run/debug/profile launch shapes, JUnit report parsing, JaCoCo coverage.
- **Phase 6** — the Spring panels and editor behaviors (config and
  annotation completion, the endpoint map, the JDK registry UI).

## How it is loaded

Compiled in, for now. FoxGarden's `crates/languages` names this crate as a
path dependency and registers it at startup — that is Phase A of the track
("open the seams, JVM still compiled in"). Phase B replaces that edge with
a manifest and a real loader, and nothing here assumes which of the two is
doing the loading.

Consequently the two checkouts have to sit side by side:

```
cross_platform_apps/
├── foxgarden/
└── spring-foxgarden/
```

## Build

```
cargo test
```

## License

MIT — see [LICENSE](LICENSE).
