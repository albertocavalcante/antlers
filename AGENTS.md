# AI Agent Guidelines for Antlers

Guidelines for AI assistants (Claude, GPT, etc.) working on this codebase.

<vision>
## One Resolver to Rule Them All

Antlers aims to be a **universal, native Rust dependency resolver** supporting
multiple package ecosystems through a unified, feature-gated architecture.

<current-state>
**Now:** JVM ecosystem (Maven, Gradle)
- Maven POM parsing, Gradle Module Metadata
- Output: Bazel, Buck2, Coursier, Gradle formats
</current-state>

<future-state>
**Goal:** Universal resolver with ecosystem plugins

| Ecosystem | Coordinate Format | Registry | Feature Flag |
|-----------|------------------|----------|--------------|
| JVM | `group:artifact:version` | Maven Central | `jvm` (default) |
| npm | `@scope/package@version` | npmjs.com | `npm` |
| PyPI | `package==version` | pypi.org | `python` |
| Cargo | `crate@version` | crates.io | `cargo` |
| NuGet | `Package.Name/version` | nuget.org | `nuget` |
| Go | `module@version` | proxy.golang.org | `go` |
</future-state>

<architecture-principles>
1. **Feature-gated ecosystems** - Compile only what you need
2. **Shared resolution core** - `dendro` handles all ecosystems
3. **Pluggable parsers** - Each ecosystem has its own metadata parser
4. **Unified lockfile** - Single format, multiple ecosystems
5. **Minimal binaries** - `antlers-resolve` stays under 2MB per ecosystem
</architecture-principles>

<binary-targets>
| Binary | Purpose | Size Target |
|--------|---------|-------------|
| `antlers` | Full CLI, all features | ~15MB |
| `antlers-resolve` | Minimal lockfile processor | ~2MB |
| `libantlers` | Embeddable library | Feature-dependent |
</binary-targets>
</vision>

<project-context>
## Current Crate Structure

Core (ecosystem-agnostic):
- `dendro` - Dependency resolution engine (works with any ecosystem)
- `gather` - HTTP fetching with caching and auth
- `antlers-lock` - Universal lockfile format and writers

JVM ecosystem (feature: `jvm`):
- `gav` - Maven coordinate parsing (group:artifact:version)
- `pomace` - POM file parsing
- `grale` - Gradle Module Metadata parsing

High-level:
- `antlers` - Unified API combining ecosystems
- `antlers-cli` - Full-featured command-line interface
- `antlers-resolve` - Minimal binary for build system integration
</project-context>

<critical-rules>

<rule id="serialization" priority="highest">
## Never Use String Concatenation for Structured Output

When generating code or structured data, ALWAYS use proper serialization libraries.

| Format | Crate | Example |
|--------|-------|---------|
| JSON | `serde_json` | `serde_json::to_string_pretty(&value)` |
| TOML | `toml` | `toml::to_string(&value)` |
| Starlark | `serde_starlark` | `serde_starlark::to_string(&value)` |
| XML | `quick-xml` | `quick_xml::se::to_string(&value)` |

<bad-example>
```rust
// WRONG: String concatenation is fragile
fn write_starlark(name: &str) -> String {
    format!("http_jar(\n    name = \"{}\",\n)", name)
}
```
</bad-example>

<good-example>
```rust
// CORRECT: Proper AST serialization
#[derive(Serialize)]
#[serde(rename = "http_jar")]
struct HttpJar {
    name: String,
    url: String,
    sha256: String,
}

fn write_starlark(jar: &HttpJar) -> Result<String> {
    serde_starlark::to_string(jar)
}
```
</good-example>

Benefits:
- Guaranteed syntactically valid output
- Automatic escaping of special characters
- Consistent formatting
- Type safety through Rust's type system
</rule>

<rule id="research-first" priority="high">
## Research Before Implementation

Before implementing ANY feature:

1. **Search for existing crates** - Check crates.io for libraries that handle the format/task
2. **Study similar projects** - Look at rules_jvm_external, rules_rust, coursier for patterns
3. **Prefer established libraries** - Don't reinvent what already exists
4. **Check the ecosystem** - Maven, Gradle, Bazel all have conventions to follow

<checklist>
- [ ] Searched crates.io for existing solutions?
- [ ] Checked how similar tools solve this?
- [ ] Using proper serialization, not string concatenation?
- [ ] Tests verify output is actually valid (parsed back)?
</checklist>
</rule>

<rule id="type-safety" priority="high">
## Type-Safe Patterns

- Use **enums** for fixed value sets (conflict strategies, output formats)
- Use **newtypes** for validated strings (coordinates, versions, checksums)
- Use **trait methods** instead of string comparisons for behavior

<bad-example>
```rust
// WRONG: String comparison for behavior
if strategy.name() == "strict" {
    fail_on_conflict();
}
```
</bad-example>

<good-example>
```rust
// CORRECT: Trait method
trait ConflictStrategy {
    fn fails_on_conflict(&self) -> bool;
}

if strategy.fails_on_conflict() {
    // ...
}
```
</good-example>
</rule>

<rule id="feature-gates" priority="high">
## Design for Feature Gates and Multi-Ecosystem

All new code MUST be designed with ecosystem extensibility in mind.

<principles>
1. **Ecosystem-specific code behind feature flags**
   - JVM code: `#[cfg(feature = "jvm")]`
   - npm code: `#[cfg(feature = "npm")]`

2. **Core traits are ecosystem-agnostic**
   - `Coordinate` trait, not `MavenCoordinate` struct in core
   - `MetadataParser` trait, not `PomParser` in core

3. **No hardcoded ecosystem assumptions**
   - Don't assume Maven Central in core code
   - Don't assume `.jar` extensions in generic code
</principles>

<good-example>
```rust
// Core trait (ecosystem-agnostic)
pub trait Coordinate: Display + FromStr {
    fn name(&self) -> &str;
    fn version(&self) -> &Version;
}

// JVM implementation (feature-gated)
#[cfg(feature = "jvm")]
impl Coordinate for MavenCoordinate {
    fn name(&self) -> &str { &self.artifact_id }
    fn version(&self) -> &Version { &self.version }
}
```
</good-example>

<bad-example>
```rust
// WRONG: Ecosystem-specific in core
pub struct Dependency {
    pub group_id: String,      // Maven-specific!
    pub artifact_id: String,   // Maven-specific!
}
```
</bad-example>
</rule>

</critical-rules>

<crate-guidelines>

<crate name="antlers-lock">
Output writers MUST:
1. Use proper serialization libraries (serde_json, serde_starlark, etc.)
2. Support both compact and pretty-printed output where applicable
3. Include tests that parse output back to verify correctness
4. Document the output format in doc comments with examples
</crate>

<crate name="dendro">
Resolution engine MUST:
1. Use trait methods for strategy behavior, not string matching
2. Be deterministic - same inputs always produce same outputs
3. Include conflict resolution tests with known edge cases
4. Handle cycles and diamond dependencies correctly
</crate>

<crate name="gather">
HTTP fetching MUST:
1. Respect cache headers and checksums
2. Support offline mode
3. Handle authentication (Basic, Bearer, netrc)
4. Provide clear error messages for network failures
</crate>

</crate-guidelines>

<testing-standards>
## Testing Requirements

1. **Unit tests** - Each public function must have tests
2. **Integration tests** - Real-world fixtures in `tests/fixtures/`
3. **Corpus tests** - Validate generated formats against known-good outputs
4. **Roundtrip tests** - Parse generated output back when possible
5. **Edge cases** - Empty inputs, special characters, unicode, large inputs
</testing-standards>

<code-style>
## Rust Code Style

- Follow `cargo clippy` with pedantic lints enabled
- Use `cargo fmt` for formatting
- Prefer `thiserror` for error types
- Use `tracing` for logging, not `println!`
- Document public APIs with examples
</code-style>

<commit-conventions>
## Commit Message Format

```
type(scope): description

[optional body]
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`
Scopes: `cli`, `lock`, `resolve`, `fetch`, `config`

Examples:
- `feat(lock): add Coursier JSON writer`
- `fix(resolve): handle cyclic dependencies`
- `docs(cli): add examples for resolve command`
</commit-conventions>
