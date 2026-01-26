//! Registry types for pluggable handlers.

pub mod format;
pub mod output;
pub mod strategy;

pub use format::{FormatHandler, FormatInfo, FormatRegistry};
pub use output::{FormatOptions, OutputFormatter, OutputInfo, OutputRegistry};
pub use strategy::{StrategyFactory, StrategyInfo, StrategyRegistry};
