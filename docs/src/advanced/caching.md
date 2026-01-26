# Caching

The current CLI does not persist an on-disk cache; it fetches artifacts on
request each run. Caching primitives exist in the Rust API for integrations.

## Library Usage

The `gather::Fetcher` supports optional caches:

```rust
use antlers::{Fetcher, FileCache, RepositoryList};

let cache = FileCache::new("/tmp/antlers-cache")?;
let fetcher = Fetcher::new(RepositoryList::with_defaults())
    .with_cache(cache);
```

## Hermetic Integrations

For build system integrations, use `CacheConfig` to define explicit, hermetic
cache paths:

```rust
use antlers::config::CacheConfig;

let cache = CacheConfig::read_only("/sandbox/cache");
```

CLI cache management commands are not available yet; see the Roadmap.
