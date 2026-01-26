//! Gradle Module Metadata (GMM) support for dependency resolution.
//!
//! This module provides support for resolving dependencies using Gradle Module
//! Metadata (.module) files, which provide richer dependency information than
//! Maven POMs including variants, capabilities, and rich version constraints.
//!
//! The [`HybridFetcher`] tries GMM first, falling back to POM if not available.

use std::sync::Arc;

use dendro::{Checksums, ProjectFetcher};
use gather::{ChecksumAlgo, Fetcher, RepositoryList};
use gav::{Artifact, Coordinates, Dependency, ManagedDependency, ParentRef, Project, Version};
use grale::{GradleModule, GradleModuleParser, Variant};
use tracing::{debug, trace, warn};

use crate::fetcher::{PomFetchError, PomFetcher, PomProject};

/// Variant selection strategy for Gradle Module Metadata.
///
/// Gradle modules contain multiple variants representing different configurations
/// of the same artifact (e.g., API vs runtime dependencies, JVM version targeting).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VariantSelection {
    /// Select the runtime variant (java-runtime usage).
    ///
    /// This is the default and includes all runtime dependencies.
    /// Most appropriate for building and running applications.
    #[default]
    Runtime,

    /// Select the API variant (java-api usage).
    ///
    /// This includes only compile-time API dependencies.
    /// Useful for building libraries where you want minimal transitive deps.
    Api,
}

impl VariantSelection {
    /// Returns the variant selection as a string.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Runtime => "runtime",
            Self::Api => "api",
        }
    }
}

/// A project backed by Gradle Module Metadata.
///
/// This implements the [`Project`] trait, providing dependencies extracted
/// from the selected variant of the GMM file.
#[derive(Debug, Clone)]
pub struct GradleModuleProject {
    /// The parsed Gradle module.
    module: GradleModule,
    /// The selected variant name.
    selected_variant: String,
    /// Coordinates (computed once).
    coordinates: Coordinates,
    /// Version (computed once).
    version: Version,
    /// Dependencies converted from the selected variant.
    dependencies: Vec<Dependency>,
    /// Managed dependencies from dependency constraints.
    managed_dependencies: Vec<ManagedDependency>,
}

impl GradleModuleProject {
    /// Creates a new `GradleModuleProject` from a parsed [`GradleModule`].
    ///
    /// Selects the appropriate variant based on the selection strategy and
    /// converts dependencies to the gav format.
    #[must_use]
    #[allow(clippy::option_if_let_else)]
    pub fn new(module: GradleModule, selection: VariantSelection) -> Self {
        let coordinates = Coordinates::new(&module.component.group, &module.component.module);
        let version = Version::new(&module.component.version);

        // Select variant based on strategy
        let variant = match selection {
            VariantSelection::Runtime => module.runtime_variant(),
            VariantSelection::Api => module.api_variant(),
        };

        let (selected_variant, dependencies, managed_dependencies) = if let Some(v) = variant {
            let deps = Self::convert_dependencies(v);
            let managed = Self::convert_constraints(v);
            (v.name.clone(), deps, managed)
        } else {
            // No matching variant, try to find any suitable variant
            let fallback = module
                .variants
                .iter()
                .find(|v| !v.is_redirect() && !v.dependencies.is_empty());

            if let Some(v) = fallback {
                debug!(
                    "No {} variant for {}, using fallback: {}",
                    selection.as_str(),
                    module.coordinates(),
                    v.name
                );
                let deps = Self::convert_dependencies(v);
                let managed = Self::convert_constraints(v);
                (v.name.clone(), deps, managed)
            } else {
                debug!(
                    "No suitable variant for {}, using empty deps",
                    module.coordinates()
                );
                (String::new(), Vec::new(), Vec::new())
            }
        };

        Self {
            module,
            selected_variant,
            coordinates,
            version,
            dependencies,
            managed_dependencies,
        }
    }

    /// Converts variant dependencies to gav Dependencies.
    fn convert_dependencies(variant: &Variant) -> Vec<Dependency> {
        variant
            .dependencies
            .iter()
            .filter_map(grale::VariantDependency::to_gav_dependency)
            .collect()
    }

    /// Converts dependency constraints to managed dependencies.
    fn convert_constraints(variant: &Variant) -> Vec<ManagedDependency> {
        variant
            .dependency_constraints
            .iter()
            .filter_map(grale::DependencyConstraint::to_managed_dependency)
            .collect()
    }

    /// Returns a reference to the underlying [`GradleModule`].
    #[must_use]
    pub const fn module(&self) -> &GradleModule {
        &self.module
    }

    /// Returns the name of the selected variant.
    #[must_use]
    pub fn selected_variant(&self) -> &str {
        &self.selected_variant
    }
}

impl Project for GradleModuleProject {
    fn coordinates(&self) -> &Coordinates {
        &self.coordinates
    }

    fn version(&self) -> &Version {
        &self.version
    }

    fn dependencies(&self) -> Vec<&Dependency> {
        self.dependencies.iter().collect()
    }

    fn managed_dependencies(&self) -> Vec<&ManagedDependency> {
        self.managed_dependencies.iter().collect()
    }

    fn parent(&self) -> Option<&ParentRef> {
        // GMM doesn't have parent inheritance like POM
        None
    }

    fn properties(&self) -> &[(String, String)] {
        // GMM doesn't have properties like POM
        &[]
    }

    fn packaging(&self) -> &'static str {
        // Determine packaging from module type
        if self.module.is_platform() {
            "pom"
        } else {
            "jar"
        }
    }
}

/// A project that can be either GMM-backed or POM-backed.
///
/// This enum allows the resolver to work transparently with either
/// Gradle Module Metadata or Maven POM projects.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum HybridProject {
    /// A project backed by Gradle Module Metadata.
    Gmm(GradleModuleProject),
    /// A project backed by Maven POM.
    Pom(Box<PomProject>),
}

impl Project for HybridProject {
    fn coordinates(&self) -> &Coordinates {
        match self {
            Self::Gmm(p) => p.coordinates(),
            Self::Pom(p) => p.as_ref().coordinates(),
        }
    }

    fn version(&self) -> &Version {
        match self {
            Self::Gmm(p) => p.version(),
            Self::Pom(p) => p.as_ref().version(),
        }
    }

    fn dependencies(&self) -> Vec<&Dependency> {
        match self {
            Self::Gmm(p) => p.dependencies(),
            Self::Pom(p) => p.as_ref().dependencies(),
        }
    }

    fn managed_dependencies(&self) -> Vec<&ManagedDependency> {
        match self {
            Self::Gmm(p) => p.managed_dependencies(),
            Self::Pom(p) => p.as_ref().managed_dependencies(),
        }
    }

    fn parent(&self) -> Option<&ParentRef> {
        match self {
            Self::Gmm(p) => p.parent(),
            Self::Pom(p) => p.as_ref().parent(),
        }
    }

    fn properties(&self) -> &[(String, String)] {
        match self {
            Self::Gmm(p) => p.properties(),
            Self::Pom(p) => p.as_ref().properties(),
        }
    }

    fn packaging(&self) -> &'static str {
        match self {
            Self::Gmm(p) => p.packaging(),
            Self::Pom(p) => p.as_ref().packaging(),
        }
    }
}

/// A fetcher that tries Gradle Module Metadata first, then falls back to POM.
///
/// This provides transparent support for both GMM and POM formats while
/// preferring GMM for its richer metadata when available.
pub struct HybridFetcher {
    fetcher: Arc<Fetcher>,
    pom_fetcher: PomFetcher,
    variant_selection: VariantSelection,
    gmm_enabled: bool,
}

impl HybridFetcher {
    /// Creates a new [`HybridFetcher`] with the given repositories.
    pub fn new(repositories: RepositoryList) -> Self {
        let fetcher = Arc::new(Fetcher::new(repositories.clone()));
        Self {
            fetcher,
            pom_fetcher: PomFetcher::new(repositories),
            variant_selection: VariantSelection::default(),
            gmm_enabled: true,
        }
    }

    /// Sets the variant selection strategy.
    #[must_use]
    pub const fn with_variant_selection(mut self, selection: VariantSelection) -> Self {
        self.variant_selection = selection;
        self
    }

    /// Enables or disables GMM support.
    ///
    /// When disabled, only POM fetching is used (for --pom-only mode).
    #[must_use]
    pub const fn with_gmm_enabled(mut self, enabled: bool) -> Self {
        self.gmm_enabled = enabled;
        self
    }

    /// Returns a reference to the underlying fetcher.
    pub fn fetcher(&self) -> &Fetcher {
        &self.fetcher
    }

    /// Attempts to fetch and parse GMM for an artifact.
    ///
    /// Returns `None` if GMM is not available or parsing fails.
    async fn try_fetch_gmm(&self, artifact: &Artifact) -> Option<GradleModuleProject> {
        // Try to fetch .module file
        let module_content = match self.fetcher.fetch_module(artifact).await {
            Ok(Some(content)) => content,
            Ok(None) => {
                trace!("No .module file for {}", artifact.coordinate());
                return None;
            }
            Err(e) => {
                warn!(
                    "Error fetching .module for {}: {}",
                    artifact.coordinate(),
                    e
                );
                return None;
            }
        };

        // Parse the module metadata
        match GradleModuleParser::parse(&module_content) {
            Ok(module) => {
                debug!(
                    "Parsed GMM for {} (variant: {})",
                    artifact.coordinate(),
                    self.variant_selection.as_str()
                );

                // Handle available_at redirects
                if let Some(redirect) = self.handle_available_at(&module, 0).await {
                    return Some(redirect);
                }

                Some(GradleModuleProject::new(module, self.variant_selection))
            }
            Err(e) => {
                warn!(
                    "Failed to parse .module for {}: {}",
                    artifact.coordinate(),
                    e
                );
                None
            }
        }
    }

    /// Handles `available_at` redirects in GMM variants.
    ///
    /// Some modules redirect to other modules for actual variant content.
    /// This follows the redirect up to a maximum depth.
    fn handle_available_at<'a>(
        &'a self,
        module: &'a GradleModule,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Option<GradleModuleProject>> + Send + 'a>>
    {
        Box::pin(async move {
            const MAX_REDIRECT_DEPTH: usize = 3;

            if depth >= MAX_REDIRECT_DEPTH {
                warn!(
                    "Maximum redirect depth ({}) exceeded for {}",
                    MAX_REDIRECT_DEPTH,
                    module.coordinates()
                );
                return None;
            }

            // Check if selected variant has available_at
            let variant = match self.variant_selection {
                VariantSelection::Runtime => module.runtime_variant(),
                VariantSelection::Api => module.api_variant(),
            };

            if let Some(v) = variant
                && let Some(ref available_at) = v.available_at
            {
                debug!(
                    "Following available_at redirect: {} -> {}",
                    module.coordinates(),
                    available_at.coordinates()
                );

                // Fetch the redirected module
                let redirect_artifact = Artifact::new(
                    &available_at.group,
                    &available_at.module,
                    &available_at.version,
                );

                // Recursively fetch and handle redirects
                if let Some(project) = self.try_fetch_gmm(&redirect_artifact).await {
                    return Some(project);
                }
            }

            None
        })
    }
}

impl ProjectFetcher for HybridFetcher {
    type Project = HybridProject;
    type Error = PomFetchError;

    async fn fetch(&self, artifact: &Artifact) -> std::result::Result<Self::Project, Self::Error> {
        // Try GMM first if enabled
        if self.gmm_enabled
            && let Some(gmm_project) = self.try_fetch_gmm(artifact).await
        {
            return Ok(HybridProject::Gmm(gmm_project));
        }

        // Fall back to POM
        let pom_project = self.pom_fetcher.fetch(artifact).await?;
        Ok(HybridProject::Pom(Box::new(pom_project)))
    }

    async fn fetch_checksums(&self, artifact: &Artifact) -> Checksums {
        let mut checksums = Checksums::none();

        // Fetch SHA1
        if let Ok(Some(sha1)) = self
            .fetcher
            .fetch_checksum(artifact, ChecksumAlgo::Sha1)
            .await
        {
            checksums = checksums.with_sha1(sha1);
        }

        // Fetch SHA256
        if let Ok(Some(sha256)) = self
            .fetcher
            .fetch_checksum(artifact, ChecksumAlgo::Sha256)
            .await
        {
            checksums = checksums.with_sha256(sha256);
        }

        // Set repository name
        if let Some(repo) = self.fetcher.repositories().iter().next() {
            checksums = checksums.with_repository(&repo.name);
        }

        checksums
    }
}

impl std::fmt::Debug for HybridFetcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HybridFetcher")
            .field("variant_selection", &self.variant_selection)
            .field("gmm_enabled", &self.gmm_enabled)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use grale::Attributes;
    use grale::{Component, VariantDependency, VersionRequirement, keys, values};

    fn create_test_module() -> GradleModule {
        let runtime_variant = Variant {
            name: "runtimeElements".to_string(),
            attributes: [
                (keys::USAGE.to_string(), values::JAVA_RUNTIME.to_string()),
                (keys::CATEGORY.to_string(), values::LIBRARY.to_string()),
            ]
            .into_iter()
            .collect(),
            dependencies: vec![VariantDependency {
                group: "com.google.guava".to_string(),
                module: "guava".to_string(),
                version: Some(VersionRequirement {
                    requires: Some("31.1-jre".to_string()),
                    strictly: None,
                    prefers: None,
                    rejects: vec![],
                }),
                reason: None,
                attributes: Attributes::default(),
                requested_capabilities: vec![],
                excludes: vec![],
                endorse_strict_versions: false,
                third_party_compatibility: None,
            }],
            dependency_constraints: vec![],
            files: vec![],
            capabilities: vec![],
            available_at: None,
        };

        let api_variant = Variant {
            name: "apiElements".to_string(),
            attributes: [
                (keys::USAGE.to_string(), values::JAVA_API.to_string()),
                (keys::CATEGORY.to_string(), values::LIBRARY.to_string()),
            ]
            .into_iter()
            .collect(),
            dependencies: vec![],
            dependency_constraints: vec![],
            files: vec![],
            capabilities: vec![],
            available_at: None,
        };

        GradleModule {
            format_version: "1.1".to_string(),
            component: Component {
                group: "org.example".to_string(),
                module: "library".to_string(),
                version: "1.0.0".to_string(),
                url: None,
                attributes: Attributes::default(),
            },
            created_by: None,
            variants: vec![api_variant, runtime_variant],
        }
    }

    #[test]
    fn test_gradle_module_project_runtime() {
        let module = create_test_module();
        let project = GradleModuleProject::new(module, VariantSelection::Runtime);

        assert_eq!(project.coordinates().group_id, "org.example");
        assert_eq!(project.coordinates().artifact_id, "library");
        assert_eq!(project.version().as_str(), "1.0.0");
        assert_eq!(project.selected_variant(), "runtimeElements");
        assert_eq!(project.dependencies().len(), 1);
        assert_eq!(project.dependencies()[0].group_id(), "com.google.guava");
    }

    #[test]
    fn test_gradle_module_project_api() {
        let module = create_test_module();
        let project = GradleModuleProject::new(module, VariantSelection::Api);

        assert_eq!(project.selected_variant(), "apiElements");
        assert!(project.dependencies().is_empty());
    }

    #[test]
    fn test_variant_selection_default() {
        assert_eq!(VariantSelection::default(), VariantSelection::Runtime);
    }

    #[test]
    fn test_hybrid_project_delegates_correctly() {
        let module = create_test_module();
        let gmm_project = GradleModuleProject::new(module, VariantSelection::Runtime);
        let hybrid = HybridProject::Gmm(gmm_project);

        assert_eq!(hybrid.coordinates().group_id, "org.example");
        assert_eq!(hybrid.version().as_str(), "1.0.0");
        assert_eq!(hybrid.dependencies().len(), 1);
    }
}
