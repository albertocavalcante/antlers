# AI Agent Guidelines for Antlers

Guidelines for AI assistants (Claude, GPT, etc.) working on this codebase.

<project-context>
Antlers is a native Rust resolver for JVM dependencies. It generates lockfiles
and output formats for Bazel, Buck2, Gradle, and other build systems.

Key crates:
- `gav` - Maven coordinate parsing (group:artifact:version)
- `pomace` - POM file parsing
- `grale` - Gradle Module Metadata parsing
- `dendro` - Dependency resolution engine
- `gather` - HTTP fetching with caching
- `antlers-lock` - Lockfile formats and writers
- `antlers` - High-level API
- `antlers-cli` - Command-line interface
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
