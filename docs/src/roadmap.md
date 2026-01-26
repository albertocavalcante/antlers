# Roadmap

This is a forward-looking list of ideas. Nothing here is implemented in the
current CLI unless stated elsewhere in the docs.

## CLI

- Lockfile commands (generate/update/read) backed by `antlers-lock`.
- Cache management commands and on-disk caching for the resolver.
- `antlers resolve` reading `antlers.toml` dependencies and resolver settings.
- Hermetic/offline flags and explicit network controls.
- Shell completion generation.

## Outputs and Integration

- Bazel-compatible output from the CLI (via `antlers-lock` V2 writer).
- More structured output formats for build system consumption.
- Expanded migration discovery (including Windows pip.ini).
