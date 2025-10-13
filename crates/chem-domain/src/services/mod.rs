//! # Domain Services
//!
//! Services contain pure business logic that coordinates domain entities and
//! uses repositories. They implement the application's core use cases while
//! maintaining domain isolation.
//!
//! ## Phase 2 Design Principles
//! - Pure business logic (no external dependencies)
//! - Use dependency injection via ports
//! - Exhaustive error handling
//! - Immutable data flow
pub mod family_service;
pub mod molecule_service;
pub use family_service::FamilyService;
pub use molecule_service::MoleculeService;
