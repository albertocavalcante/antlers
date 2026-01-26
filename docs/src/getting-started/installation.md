# Installation

## From Source (Cargo)

If you have Rust installed, you can build from source:

```bash
cargo install --git https://github.com/albertocavalcante/antlers antlers-cli
```

## Pre-built Binaries

Pre-built binaries are available on the
[GitHub Releases](https://github.com/albertocavalcante/antlers/releases) page
for:

- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)

### Linux / macOS

```bash
# Download the latest release (adjust URL for your platform)
curl -LO https://github.com/albertocavalcante/antlers/releases/latest/download/antlers-linux-x86_64.tar.gz

# Extract
tar xzf antlers-linux-x86_64.tar.gz

# Move to PATH
sudo mv antlers /usr/local/bin/
```

### Windows

Download the `.zip` file from the releases page and add the extracted directory
to your `PATH`.

## Verify Installation

```bash
antlers --version
```

## Next Steps

Once installed, head to the [Quick Start](quickstart.md) guide to resolve your
first dependency.
