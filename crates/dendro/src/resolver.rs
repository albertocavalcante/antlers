//! The main dependency resolver.
//!
//! This module provides the [`Resolver`] struct which implements the format-agnostic
//! dependency resolution algorithm.

use std::collections::{HashMap, HashSet};
use std::future::Future;

use gav::{Artifact, Exclusions, Project, Version, VersionConstraint};
use tracing::{debug, info, trace, warn};

use crate::{
    ConflictStrategy, Error, NearestWins, Resolution, ResolvedArtifact, ResolverConfig, Result,
    VersionChoice, VersionConflict,
};

/// Checksums for an artifact.
#[derive(Debug, Clone, Default)]
pub struct Checksums {
    /// SHA1 checksum.
    pub sha1: Option<String>,
    /// SHA256 checksum.
    pub sha256: Option<String>,
    /// Repository where the artifact was found.
    pub repository: Option<String>,
}

impl Checksums {
    /// Creates empty checksums.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// Creates checksums with SHA1.
    #[must_use]
    pub fn with_sha1(mut self, sha1: impl Into<String>) -> Self {
        self.sha1 = Some(sha1.into());
        self
    }

    /// Creates checksums with SHA256.
    #[must_use]
    pub fn with_sha256(mut self, sha256: impl Into<String>) -> Self {
        self.sha256 = Some(sha256.into());
        self
    }

    /// Sets the repository.
    #[must_use]
    pub fn with_repository(mut self, repository: impl Into<String>) -> Self {
        self.repository = Some(repository.into());
        self
    }
}

/// Trait for fetching project metadata (POM, GMM, etc.).
///
/// Implementors of this trait provide the ability to fetch project metadata
/// from repositories. This allows the resolver to be format-agnostic.
pub trait ProjectFetcher: Send + Sync {
    /// The project type returned by this fetcher.
    type Project: Project + Send + Sync;

    /// The error type for fetch operations.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Fetches the project metadata for an artifact.
    ///
    /// This may involve network requests to Maven repositories or reading
    /// from a local cache.
    fn fetch(
        &self,
        artifact: &Artifact,
    ) -> impl Future<Output = std::result::Result<Self::Project, Self::Error>> + Send;

    /// Fetches checksums for an artifact.
    ///
    /// Returns empty checksums if not available.
    fn fetch_checksums(&self, artifact: &Artifact) -> impl Future<Output = Checksums> + Send;
}

/// The dependency resolver.
///
/// This resolver implements Maven-style dependency resolution with support for:
/// - Transitive dependency resolution
/// - Version conflict resolution with pluggable strategies
/// - Dependency exclusions
/// - Configurable scope filtering
///
/// # Type Parameters
///
/// - `F`: The [`ProjectFetcher`] implementation used to fetch project metadata
/// - `S`: The [`ConflictStrategy`] used to resolve version conflicts
///
/// # Example
///
/// ```ignore
/// use dendro::{Resolver, ResolverConfig, NearestWins};
///
/// let fetcher = MyProjectFetcher::new();
/// let resolver = Resolver::new(fetcher)
///     .with_config(ResolverConfig::new().transitive(true));
///
/// let artifact = Artifact::parse("com.example:lib:1.0.0").unwrap();
/// let resolution = resolver.resolve(&artifact).await?;
/// ```
pub struct Resolver<F: ProjectFetcher, S: ConflictStrategy = NearestWins> {
    fetcher: F,
    strategy: S,
    config: ResolverConfig,
}

impl<F: ProjectFetcher> Resolver<F, NearestWins> {
    /// Creates a new resolver with the default conflict strategy (nearest wins).
    pub fn new(fetcher: F) -> Self {
        Self {
            fetcher,
            strategy: NearestWins,
            config: ResolverConfig::default(),
        }
    }
}

impl<F: ProjectFetcher, S: ConflictStrategy> Resolver<F, S> {
    /// Sets the resolver configuration.
    #[must_use]
    pub fn with_config(mut self, config: ResolverConfig) -> Self {
        self.config = config;
        self
    }

    /// Sets the conflict resolution strategy.
    #[must_use]
    pub fn with_strategy<S2: ConflictStrategy>(self, strategy: S2) -> Resolver<F, S2> {
        Resolver {
            fetcher: self.fetcher,
            strategy,
            config: self.config,
        }
    }

    /// Returns a reference to the configuration.
    pub const fn config(&self) -> &ResolverConfig {
        &self.config
    }

    /// Resolves an artifact and its dependencies.
    ///
    /// This method performs the full dependency resolution algorithm:
    /// 1. Starts with the root artifact
    /// 2. Fetches its project metadata
    /// 3. Processes its dependencies transitively (if enabled)
    /// 4. Handles version conflicts using the configured strategy
    /// 5. Applies exclusions
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The maximum resolution depth is exceeded
    /// - A circular dependency is detected
    /// - The strict conflict strategy is used and a conflict is detected
    #[allow(clippy::too_many_lines)]
    pub async fn resolve(&self, artifact: &Artifact) -> Result<Resolution> {
        info!("Resolving {}", artifact);

        let mut resolution = Resolution::new(artifact.clone());
        let mut visited: HashSet<String> = HashSet::new();

        // Track version selections for conflict resolution
        // Key: groupId:artifactId, Value: (Version, depth)
        let mut version_selections: HashMap<String, (Version, usize)> = HashMap::new();

        // Queue entries: (artifact, depth, exclusions from parent)
        let mut queue: Vec<(Artifact, usize, Exclusions)> =
            vec![(artifact.clone(), 0, Exclusions::none())];

        while let Some((current, depth, exclusions)) = queue.pop() {
            // Check depth limit
            if depth > self.config.max_depth {
                return Err(Error::MaxDepthExceeded(depth));
            }

            let ga_key = format!(
                "{}:{}",
                current.coordinates.group_id, current.coordinates.artifact_id
            );
            let coord = current.coordinate();

            // Check if this artifact is excluded
            if exclusions.matches(&current) {
                debug!("Skipping excluded artifact: {}", current);
                continue;
            }

            // Check for version conflicts
            if let Some((selected_version, selected_depth)) = version_selections.get(&ga_key) {
                let choice = self.strategy.resolve(
                    selected_version,
                    *selected_depth,
                    &current.version,
                    depth,
                );

                match choice {
                    VersionChoice::KeepExisting => {
                        if selected_version != &current.version {
                            debug!(
                                "Version conflict for {}: keeping {} (depth {}) over {} (depth {})",
                                ga_key, selected_version, selected_depth, current.version, depth
                            );

                            // Check for strict strategy
                            if self.is_strict_strategy() {
                                return Err(Error::VersionConflict {
                                    artifact: ga_key.clone(),
                                    details: format!(
                                        "existing: {} (depth {}), new: {} (depth {})",
                                        selected_version, selected_depth, current.version, depth
                                    ),
                                });
                            }

                            resolution.add_conflict(VersionConflict::new(
                                &ga_key,
                                vec![selected_version.to_string(), current.version.to_string()],
                                selected_version.to_string(),
                                self.strategy.name(),
                            ));
                        }
                        continue;
                    }
                    VersionChoice::UseNew => {
                        debug!(
                            "Version conflict for {}: using {} (depth {}) over {} (depth {})",
                            ga_key, current.version, depth, selected_version, selected_depth
                        );

                        // Check for strict strategy
                        if self.is_strict_strategy() {
                            return Err(Error::VersionConflict {
                                artifact: ga_key.clone(),
                                details: format!(
                                    "existing: {} (depth {}), new: {} (depth {})",
                                    selected_version, selected_depth, current.version, depth
                                ),
                            });
                        }

                        resolution.add_conflict(VersionConflict::new(
                            &ga_key,
                            vec![selected_version.to_string(), current.version.to_string()],
                            current.version.to_string(),
                            self.strategy.name(),
                        ));
                        // Fall through to process the new version
                    }
                }
            }

            // Record this version selection
            version_selections.insert(ga_key.clone(), (current.version.clone(), depth));

            // Skip if already visited this exact coordinate
            if visited.contains(&coord) {
                continue;
            }
            visited.insert(coord.clone());

            debug!("Processing {} (depth {})", current, depth);

            // Fetch project metadata
            let project = match self.fetcher.fetch(&current).await {
                Ok(p) => p,
                Err(e) => {
                    warn!("Failed to fetch project for {}: {}", current, e);
                    continue;
                }
            };

            // Fetch checksums
            let checksums = self.fetcher.fetch_checksums(&current).await;

            // Add to resolution
            let mut resolved = ResolvedArtifact::new(current.clone());
            if let Some(sha1) = checksums.sha1 {
                resolved = resolved.with_sha1(sha1);
            }
            if let Some(sha256) = checksums.sha256 {
                resolved = resolved.with_sha256(sha256);
            }
            if let Some(repo) = checksums.repository {
                resolved = resolved.with_repository(repo);
            }
            resolution.add_artifact(resolved);

            // Process dependencies if transitive resolution is enabled
            if self.config.transitive {
                self.queue_dependencies(&project, depth, &exclusions, &visited, &mut queue);
            }
        }

        info!(
            "Resolution complete: {} artifacts, {} conflicts",
            resolution.len(),
            resolution.conflict_count()
        );

        Ok(resolution)
    }

    /// Queues dependencies for processing.
    #[allow(clippy::single_match_else)]
    fn queue_dependencies(
        &self,
        project: &F::Project,
        depth: usize,
        parent_exclusions: &Exclusions,
        visited: &HashSet<String>,
        queue: &mut Vec<(Artifact, usize, Exclusions)>,
    ) {
        for dep in project.dependencies() {
            // Skip based on scope
            if !self.config.should_include_scope(&dep.scope) {
                trace!("Skipping {} - scope {:?}", dep, dep.scope);
                continue;
            }

            // Skip optional dependencies unless configured to include them
            if dep.optional && !self.config.include_optional {
                trace!("Skipping optional dependency: {}", dep);
                continue;
            }

            // Get the version - skip if no version can be determined
            let version = match &dep.version {
                Some(vc) => match vc {
                    VersionConstraint::Exact(v) => v.clone(),
                    // For ranges, we'd need to resolve the best version
                    // For now, just skip non-exact versions
                    _ => {
                        warn!("Skipping {} - non-exact version constraint: {}", dep, vc);
                        continue;
                    }
                },
                None => {
                    // Try to get version from managed dependencies
                    let managed = project.managed_dependencies();
                    let key = format!("{}:{}", dep.group_id(), dep.artifact_id());

                    let managed_version = managed.iter().find_map(|m| {
                        if m.key() == key {
                            m.version.as_ref().and_then(|vc| match vc {
                                VersionConstraint::Exact(v) => Some(v.clone()),
                                _ => None,
                            })
                        } else {
                            None
                        }
                    });

                    match managed_version {
                        Some(v) => v,
                        None => {
                            warn!("Skipping {} - no version found", dep);
                            continue;
                        }
                    }
                }
            };

            // Skip if version contains unresolved properties
            if version.as_str().contains("${") {
                warn!(
                    "Skipping {} - unresolved version property: {}",
                    dep,
                    version.as_str()
                );
                continue;
            }

            let dep_artifact = Artifact::new(dep.group_id(), dep.artifact_id(), version.as_str());
            let dep_coord = dep_artifact.coordinate();

            if !visited.contains(&dep_coord) {
                // Merge exclusions: parent's exclusions + this dependency's exclusions
                let merged_exclusions = parent_exclusions.join(&dep.exclusions);

                trace!("Queuing dependency: {} (depth {})", dep_artifact, depth + 1);
                queue.push((dep_artifact, depth + 1, merged_exclusions));
            }
        }
    }

    /// Returns true if using the strict conflict strategy.
    fn is_strict_strategy(&self) -> bool {
        self.strategy.name() == "strict"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HighestWins;
    use gav::Coordinates;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock project for testing
    #[derive(Debug, Clone)]
    struct MockProject {
        artifact: Artifact,
        deps: Vec<gav::Dependency>,
    }

    impl Project for MockProject {
        fn coordinates(&self) -> &Coordinates {
            &self.artifact.coordinates
        }

        fn version(&self) -> &Version {
            &self.artifact.version
        }

        fn dependencies(&self) -> Vec<&gav::Dependency> {
            self.deps.iter().collect()
        }

        fn managed_dependencies(&self) -> Vec<&gav::ManagedDependency> {
            Vec::new()
        }

        fn parent(&self) -> Option<&gav::ParentRef> {
            None
        }

        fn properties(&self) -> &[(String, String)] {
            &[]
        }
    }

    // Mock fetcher for testing
    struct MockFetcher {
        projects: Arc<Mutex<HashMap<String, MockProject>>>,
    }

    impl MockFetcher {
        fn new() -> Self {
            Self {
                projects: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        async fn add_project(&self, project: MockProject) {
            let mut projects = self.projects.lock().await;
            projects.insert(project.artifact.coordinate(), project);
        }
    }

    impl ProjectFetcher for MockFetcher {
        type Project = MockProject;
        type Error = std::io::Error;

        async fn fetch(
            &self,
            artifact: &Artifact,
        ) -> std::result::Result<Self::Project, Self::Error> {
            let projects = self.projects.lock().await;
            projects
                .get(&artifact.coordinate())
                .cloned()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "not found"))
        }

        async fn fetch_checksums(&self, _artifact: &Artifact) -> Checksums {
            Checksums::none()
        }
    }

    #[tokio::test]
    async fn test_resolve_single_artifact() {
        let fetcher = MockFetcher::new();
        let project = MockProject {
            artifact: Artifact::new("com.example", "lib", "1.0.0"),
            deps: vec![],
        };
        fetcher.add_project(project).await;

        let resolver = Resolver::new(fetcher);
        let artifact = Artifact::new("com.example", "lib", "1.0.0");
        let resolution = resolver.resolve(&artifact).await.unwrap();

        assert_eq!(resolution.len(), 1);
        assert!(!resolution.has_conflicts());
    }

    #[tokio::test]
    async fn test_resolve_with_dependency() {
        let fetcher = MockFetcher::new();

        // Root project with one dependency
        let dep = gav::Dependency::new(Coordinates::new("com.example", "dep"))
            .with_version(VersionConstraint::Exact(Version::new("2.0.0")));

        let root = MockProject {
            artifact: Artifact::new("com.example", "root", "1.0.0"),
            deps: vec![dep],
        };

        let dep_project = MockProject {
            artifact: Artifact::new("com.example", "dep", "2.0.0"),
            deps: vec![],
        };

        fetcher.add_project(root).await;
        fetcher.add_project(dep_project).await;

        let resolver = Resolver::new(fetcher);
        let artifact = Artifact::new("com.example", "root", "1.0.0");
        let resolution = resolver.resolve(&artifact).await.unwrap();

        assert_eq!(resolution.len(), 2);
    }

    #[tokio::test]
    async fn test_non_transitive_resolution() {
        let fetcher = MockFetcher::new();

        let dep = gav::Dependency::new(Coordinates::new("com.example", "dep"))
            .with_version(VersionConstraint::Exact(Version::new("2.0.0")));

        let root = MockProject {
            artifact: Artifact::new("com.example", "root", "1.0.0"),
            deps: vec![dep],
        };

        fetcher.add_project(root).await;

        let resolver = Resolver::new(fetcher).with_config(ResolverConfig::new().transitive(false));
        let artifact = Artifact::new("com.example", "root", "1.0.0");
        let resolution = resolver.resolve(&artifact).await.unwrap();

        // Only root should be resolved
        assert_eq!(resolution.len(), 1);
    }

    #[tokio::test]
    async fn test_conflict_resolution_nearest_wins() {
        let fetcher = MockFetcher::new();

        // A -> B:1.0 and A -> C -> B:2.0
        // With nearest-wins, B:1.0 should win (depth 1 < depth 2)

        let b_dep_1 = gav::Dependency::new(Coordinates::new("com.example", "b"))
            .with_version(VersionConstraint::Exact(Version::new("1.0")));
        let c_dep = gav::Dependency::new(Coordinates::new("com.example", "c"))
            .with_version(VersionConstraint::Exact(Version::new("1.0")));

        let root = MockProject {
            artifact: Artifact::new("com.example", "a", "1.0"),
            deps: vec![b_dep_1, c_dep],
        };

        let b_dep_2 = gav::Dependency::new(Coordinates::new("com.example", "b"))
            .with_version(VersionConstraint::Exact(Version::new("2.0")));

        let c = MockProject {
            artifact: Artifact::new("com.example", "c", "1.0"),
            deps: vec![b_dep_2],
        };

        let b1 = MockProject {
            artifact: Artifact::new("com.example", "b", "1.0"),
            deps: vec![],
        };

        let b2 = MockProject {
            artifact: Artifact::new("com.example", "b", "2.0"),
            deps: vec![],
        };

        fetcher.add_project(root).await;
        fetcher.add_project(c).await;
        fetcher.add_project(b1).await;
        fetcher.add_project(b2).await;

        let resolver = Resolver::new(fetcher);
        let artifact = Artifact::new("com.example", "a", "1.0");
        let resolution = resolver.resolve(&artifact).await.unwrap();

        // Should have recorded a conflict
        assert!(resolution.has_conflicts());
        assert_eq!(resolution.conflict_count(), 1);

        // With nearest-wins, the version at depth 1 should win
        let conflict = &resolution.conflicts[0];
        assert_eq!(conflict.artifact, "com.example:b");
        assert_eq!(conflict.strategy, "nearest-wins");

        // The selected version in the conflict should be 1.0 (nearer depth)
        assert_eq!(conflict.selected, "1.0");
    }

    #[tokio::test]
    async fn test_conflict_resolution_highest_wins() {
        let fetcher = MockFetcher::new();

        let b_dep_1 = gav::Dependency::new(Coordinates::new("com.example", "b"))
            .with_version(VersionConstraint::Exact(Version::new("1.0")));
        let c_dep = gav::Dependency::new(Coordinates::new("com.example", "c"))
            .with_version(VersionConstraint::Exact(Version::new("1.0")));

        let root = MockProject {
            artifact: Artifact::new("com.example", "a", "1.0"),
            deps: vec![b_dep_1, c_dep],
        };

        let b_dep_2 = gav::Dependency::new(Coordinates::new("com.example", "b"))
            .with_version(VersionConstraint::Exact(Version::new("2.0")));

        let c = MockProject {
            artifact: Artifact::new("com.example", "c", "1.0"),
            deps: vec![b_dep_2],
        };

        let b1 = MockProject {
            artifact: Artifact::new("com.example", "b", "1.0"),
            deps: vec![],
        };

        let b2 = MockProject {
            artifact: Artifact::new("com.example", "b", "2.0"),
            deps: vec![],
        };

        fetcher.add_project(root).await;
        fetcher.add_project(c).await;
        fetcher.add_project(b1).await;
        fetcher.add_project(b2).await;

        let resolver = Resolver::new(fetcher).with_strategy(HighestWins);
        let artifact = Artifact::new("com.example", "a", "1.0");
        let resolution = resolver.resolve(&artifact).await.unwrap();

        // B:2.0 should be selected with highest-wins
        let b = resolution
            .artifacts()
            .iter()
            .find(|a| a.artifact.coordinates.artifact_id == "b")
            .unwrap();
        assert_eq!(b.artifact.version.as_str(), "2.0");
    }
}
