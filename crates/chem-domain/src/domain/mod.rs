//! Domain Module
//!
//! This module contains the core domain logic including entities,
//! value objects, events, and ports. It represents the heart of the
//! business logic and is independent of external concerns.

pub mod entities;
pub mod events;
pub mod ports;
pub mod value_objects;

// Re-exports for convenience
pub use entities::*;
pub use events::*;
pub use ports::*;
pub use value_objects::*;
