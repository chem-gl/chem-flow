//! Domain Entities Module
//!
//! This module contains all domain entities - the core business objects
//! that have identity and encapsulate business logic and invariants.

pub mod molecule;
pub mod molecule_family;

// Re-exports for convenience
pub use molecule::*;
pub use molecule_family::*;
