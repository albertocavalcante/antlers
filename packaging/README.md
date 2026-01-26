# Packaging Templates

This directory contains packaging templates for building distribution packages.

## Structure

```
packaging/
├── deb/
│   └── control.in     # Debian/Ubuntu package control file template
├── rpm/
│   └── antlers.spec.in # RPM spec file template
└── README.md
```

## Template Placeholders

Templates use `@@PLACEHOLDER@@` syntax for substitution:

| Placeholder             | Description            | Example                                       |
| ----------------------- | ---------------------- | --------------------------------------------- |
| `@@BINARY_NAME@@`       | Binary name            | `antlers`                                     |
| `@@VERSION@@`           | Package version        | `1.0.0` or `0.0.0~nightly.20240101.abc1234`   |
| `@@RELEASE@@`           | RPM release number     | `1` or `0.nightly.20240101.abc1234`           |
| `@@ARCH@@`              | Architecture (DEB)     | `amd64`, `arm64`                              |
| `@@SUMMARY_SUFFIX@@`    | Suffix for summary     | ` (nightly)` or empty                         |
| `@@DESCRIPTION_EXTRA@@` | Extra description line | `This is an unstable nightly build.` or empty |

## Usage

Templates are processed with `sed` during CI builds:

```bash
# Example for nightly DEB
sed -e "s/@@BINARY_NAME@@/antlers/g" \
    -e "s/@@VERSION@@/0.0.0~nightly.20240101.abc1234/g" \
    -e "s/@@ARCH@@/amd64/g" \
    -e "s/@@SUMMARY_SUFFIX@@/ (nightly)/g" \
    -e "s/@@DESCRIPTION_EXTRA@@/ This is an unstable nightly build./g" \
    packaging/deb/control.in > pkg/DEBIAN/control
```

## Adding New Package Formats

To add support for new package formats (e.g., Homebrew, Alpine APK):

1. Create a new directory: `packaging/<format>/`
2. Add template files with `.in` extension
3. Update the relevant workflow to process the template
