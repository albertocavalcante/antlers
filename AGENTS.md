# Claude Code Guidelines for Antlers

This document contains guidelines for AI agents working on the Antlers codebase.

## Core Principles

### 1. Use Proper Serialization Libraries

**Never use string concatenation for structured output formats.**

When generating code or structured data, always use appropriate serialization libraries:

| Format | Library | Crate |
|--------|---------|-------|
| JSON | serde_json | `serde_json` |
| TOML | toml | `toml` |
| Starlark/Bazel | serde_starlark | `serde_starlark` |
| XML | quick-xml | `quick-xml` |
| YAML | serde_yaml | `serde_yaml` |

**Why?**
- Guarantees syntactically valid output
- Proper escaping of special characters
- Consistent formatting
- Type safety through Rust's type system
- Fewer bugs from manual string manipulation

**Bad:**
```rust
fn write_starlark(name: &str) -> String {
    format!("http_jar(\n    name = \"{}\",\n)", name)  // WRONG!
}
```

**Good:**
```rust
#[derive(Serialize)]
#[serde(rename = "http_jar")]
struct HttpJar {
    name: String,
    url: String,
    sha256: String,
}

fn write_starlark(jar: &HttpJar) -> Result<String> {
    serde_starlark::to_string(jar)  // CORRECT!
}
```

### 2. Research Before Implementation

Before implementing any output format:
1. Search for existing Rust crates that handle the format
2. Check how similar projects (rules_jvm_external, rules_rust) do it
3. Prefer established libraries over custom implementations

### 3. Type-Safe Patterns

- Use enums for fixed sets of values (conflict strategies, output formats)
- Use newtypes for validated strings (coordinates, versions)
- Prefer `impl Trait` methods over string comparisons

### 4. Testing Standards

- Unit tests for each public function
- Integration tests with real-world fixtures
- Corpus tests for format validation
- Parse generated output back when possible to verify correctness

## Crate-Specific Guidelines

### antlers-lock

Output writers must:
1. Use proper serialization libraries (serde_json, serde_starlark, etc.)
2. Support both compact and pretty-printed output
3. Include tests that verify output can be parsed by target tools
4. Document the output format in doc comments

### dendro (resolver)

- Use trait methods for strategy behavior, not string matching
- All resolution algorithms must be deterministic
- Include conflict resolution tests with known edge cases

## Pre-Implementation Checklist

Before writing code, verify:
- [ ] Is there an existing crate for this format/task?
- [ ] Have I checked how similar tools solve this problem?
- [ ] Am I using proper serialization, not string concatenation?
- [ ] Do my tests verify the output is actually valid?
