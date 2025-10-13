//! Domain Ports Module
//!
//! This module contains all the ports (interfaces) that define how the domain
//! interacts with external systems. Following the Dependency Inversion
//! Principle, the domain defines these contracts and external adapters
//! implement them.

pub mod events;
pub mod providers;
pub mod repositories;

// Re-exports for convenience
pub use events::*;
pub use providers::*;
pub use repositories::*;

// Add async-trait dependency for convenience
pub use async_trait::async_trait;
