# Caching

Antlers caches downloaded artifacts to speed up resolution and support
offline/hermetic builds.

## Default Cache Location

By default, Antlers uses platform-specific cache directories:

| Platform | Default Path                   |
| -------- | ------------------------------ |
| Linux    | `~/.cache/antlers`             |
| macOS    | `~/Library/Caches/antlers`     |
| Windows  | `%LOCALAPPDATA%\antlers\cache` |

## Custom Cache Path

### In Configuration

```toml
[cache]
path = ".antlers/cache"
```

### Via Environment Variable

```bash
export ANTLERS_CACHE_DIR=/path/to/cache
antlers resolve com.google.guava:guava:33.0.0-jre
```

## Cache Modes

Configure cache behavior:

```toml
[cache]
mode = "read-write"  # default
```

### read-write (default)

Normal caching - reads from cache, writes new artifacts:

```toml
[cache]
mode = "read-write"
```

### read-only

Uses cache but doesn't update it. Useful for hermetic builds with a vendored
cache:

```toml
[cache]
mode = "read-only"
```

### disabled

No caching - always fetches from network:

```toml
[cache]
mode = "disabled"
```

## Cache Structure

The cache stores artifacts by coordinate:

```
~/.cache/antlers/
├── maven/
│   ├── com/
│   │   └── google/
│   │       └── guava/
│   │           └── guava/
│   │               └── 33.0.0-jre/
│   │                   ├── guava-33.0.0-jre.jar
│   │                   ├── guava-33.0.0-jre.jar.sha256
│   │                   ├── guava-33.0.0-jre.pom
│   │                   └── guava-33.0.0-jre.module
│   └── org/
│       └── jetbrains/
│           └── ...
└── metadata/
    └── ...
```

## Cache Management

### View Cache Location

```bash
antlers cache path
```

### Clear Cache

```bash
antlers cache clear
```

### Cache Size

```bash
antlers cache size
```

## Vendoring for Offline Builds

For fully offline/hermetic builds, vendor the cache into your repository:

```bash
# 1. Resolve all dependencies (populates cache)
antlers resolve --all

# 2. Copy cache to project
mkdir -p .antlers
cp -r ~/.cache/antlers .antlers/cache

# 3. Configure to use vendored cache
cat >> antlers.toml <<EOF
[cache]
path = ".antlers/cache"
mode = "read-only"
EOF

# 4. Commit
git add .antlers/cache antlers.toml
git commit -m "Vendor dependency cache"
```

## CI Caching

### GitHub Actions

```yaml
- name: Cache Antlers
  uses: actions/cache@v4
  with:
    path: ~/.cache/antlers
    key: antlers-${{ hashFiles('antlers.lock') }}
    restore-keys: |
      antlers-

- name: Resolve dependencies
  run: antlers resolve --all
```

### GitLab CI

```yaml
cache:
  key: antlers-${CI_COMMIT_REF_SLUG}
  paths:
    - .antlers/cache

resolve:
  script:
    - antlers resolve --all
```

## Checksum Verification

All cached artifacts are verified by checksum:

1. SHA-256 is preferred
2. Falls back to SHA-1 if SHA-256 unavailable
3. Fails if checksum doesn't match

```
Error: SHA256 mismatch for guava-33.0.0-jre.jar
  expected: abc123...
  actual:   def456...
```

## Troubleshooting

### Cache corruption

If you suspect cache corruption, clear and repopulate:

```bash
antlers cache clear
antlers resolve --all
```

### Disk space

Check cache size:

```bash
du -sh ~/.cache/antlers
```

Clear old entries:

```bash
antlers cache gc --older-than 30d
```
