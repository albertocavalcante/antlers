# Migration Guide

Antlers can import repository and credential configurations from other package
managers, making it easy to switch from existing tools.

## Supported Formats

| Format | File                    | Description                |
| ------ | ----------------------- | -------------------------- |
| Maven  | `settings.xml`          | Servers, mirrors, profiles |
| Gradle | `settings.gradle(.kts)` | Repository declarations    |
| Ivy    | `ivysettings.xml`       | Resolvers, credentials     |
| npm    | `.npmrc`                | Registry configuration     |
| pip    | `pip.conf` / `pip.ini`  | Index URLs                 |

## Quick Migration

### Auto-Detection

Scan all standard locations and import everything found:

```bash
antlers init --detect
```

This searches:

- `~/.m2/settings.xml`
- `./settings.gradle`, `./settings.gradle.kts`
- `~/.ivy2/ivysettings.xml`
- `~/.npmrc`, `./.npmrc`
- `~/.config/pip/pip.conf`, `~/.pip/pip.conf`

### From Specific File

```bash
antlers init --from ~/.m2/settings.xml
antlers init --from settings.gradle.kts
antlers init --from ~/.ivy2/ivysettings.xml
```

## What Gets Migrated

### Repositories

- Repository URLs
- Repository IDs and names
- Ecosystem type (Maven, npm, etc.)

### Credentials

- Username/password (basic auth)
- Tokens (bearer auth)
- Environment variable references

### Environment Variables

- Patterns like `${env.VAR_NAME}` are preserved
- Migrated variables are added to `[env].allow`

## Post-Migration

After migration, review the generated `antlers.toml`:

```bash
antlers show
```

Format for consistency:

```bash
antlers fmt
```

## Detailed Guides

- [From Maven](migration-maven.md)
- [From Gradle](migration-gradle.md)
- [From Ivy](migration-ivy.md)
