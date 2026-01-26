//! Strategy registry for conflict resolution strategies.

use std::collections::HashMap;
use std::sync::Arc;

use dendro::{ConflictStrategy, HighestWins, NearestWins, StrictFails};

/// Metadata about a conflict strategy.
#[derive(Debug, Clone)]
pub struct StrategyInfo {
    /// Short identifier (e.g., "nearest-wins").
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Short description.
    pub description: &'static str,
}

/// Factory for constructing conflict strategies.
pub trait StrategyFactory: Send + Sync {
    /// Returns information about this strategy.
    fn info(&self) -> StrategyInfo;

    /// Creates a new strategy instance.
    fn create(&self) -> Box<dyn ConflictStrategy>;
}

/// Registry of strategy factories.
pub struct StrategyRegistry {
    factories: HashMap<&'static str, Arc<dyn StrategyFactory>>,
}

impl StrategyRegistry {
    /// Creates a registry with default strategies.
    #[must_use]
    pub fn with_defaults() -> Self {
        let mut registry = Self {
            factories: HashMap::new(),
        };

        registry.register(Arc::new(HighestWinsFactory));
        registry.register(Arc::new(NearestWinsFactory));
        registry.register(Arc::new(StrictFailsFactory));

        registry
    }

    /// Registers a strategy factory.
    pub fn register(&mut self, factory: Arc<dyn StrategyFactory>) {
        let info = factory.info();
        self.factories.insert(info.id, factory);
    }

    /// Returns the strategy factory for the given ID.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&dyn StrategyFactory> {
        self.factories.get(id).map(AsRef::as_ref)
    }

    /// Constructs a strategy instance by ID.
    #[must_use]
    pub fn create(&self, id: &str) -> Option<Box<dyn ConflictStrategy>> {
        self.factories.get(id).map(|f| f.create())
    }
}

struct HighestWinsFactory;

impl StrategyFactory for HighestWinsFactory {
    fn info(&self) -> StrategyInfo {
        StrategyInfo {
            id: "highest-wins",
            name: "Highest Wins",
            description: "Chooses the highest version when conflicts occur",
        }
    }

    fn create(&self) -> Box<dyn ConflictStrategy> {
        Box::new(HighestWins)
    }
}

struct NearestWinsFactory;

impl StrategyFactory for NearestWinsFactory {
    fn info(&self) -> StrategyInfo {
        StrategyInfo {
            id: "nearest-wins",
            name: "Nearest Wins",
            description: "Chooses the nearest version in the dependency graph",
        }
    }

    fn create(&self) -> Box<dyn ConflictStrategy> {
        Box::new(NearestWins)
    }
}

struct StrictFailsFactory;

impl StrategyFactory for StrictFailsFactory {
    fn info(&self) -> StrategyInfo {
        StrategyInfo {
            id: "strict",
            name: "Strict",
            description: "Fails the resolution on any version conflict",
        }
    }

    fn create(&self) -> Box<dyn ConflictStrategy> {
        Box::new(StrictFails)
    }
}
