// lib.rs
//! # Chem Domain - Pure Domain Core
//!
//! This crate implements the domain layer of a chemical information management
//! system following **Hexagonal Architecture** (Ports and Adapters) and **SOLID
//! principles**.
//!
//! ## Architecture Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    🏛️ DOMAIN CORE                           │
//! │                                                             │
//! │  📱 Application Layer (Use Cases)                          │
//! │  ├── Commands (Write Operations)                           │
//! │  └── Queries (Read Operations)                             │
//! │                                                             │
//! │  🎯 Domain Layer                                           │
//! │  ├── Entities (Molecule, MoleculeFamily)                   │
//! │  ├── Value Objects (InChIKey, SMILES, InChI)              │
//! │  ├── Events (Domain Events)                                │
//! │  └── Ports (Interfaces to external world)                  │
//! │                                                             │
//! │  🔌 Ports (Dependency Inversion)                           │
//! │  ├── Repositories (Persistence contracts)                  │
//! │  ├── Providers (External service contracts)                │
//! │  └── Events (Event publishing contracts)                   │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Design Principles
//!
//! - **Single Responsibility**: Each module has one reason to change
//! - **Open/Closed**: Extensible via strategy patterns and polymorphism
//! - **Liskov Substitution**: All implementations are truly substitutable
//! - **Interface Segregation**: Small, focused interfaces
//! - **Dependency Inversion**: Domain depends only on abstractions
//!
//! ## Usage
//!
//! ```rust,no_run
//! use chem_domain::prelude::*;
//!
//! // Create value objects with type safety
//! let inchikey = InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N")?;
//! let smiles = Smiles::new("CCO")?;
//! let inchi = InChI::new("InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3")?;
//!
//! // Build entities with validation
//! let molecule = MoleculeBuilder::new()
//!     .inchikey(inchikey)
//!     .smiles(smiles)
//!     .inchi(inchi)
//!     .metadata(serde_json::json!({"name": "ethanol"}))
//!     .build()?;
//!
//! // Use commands for write operations
//! let command = CreateMoleculeFromSmiles::new("CCO")
//!     .with_metadata(serde_json::json!({"source": "user_input"}));
//!
//! // Use queries for read operations
//! let query = GetMoleculeByInChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N")
//!     .with_properties();
//! # Ok::<(), chem_domain::DomainError>(())
//! ```

// === Core Domain ===
pub mod domain;

// === Application Layer ===
pub mod application;

// === Legacy Support (Phase 4: Remove) ===
mod domain_stubs;
mod errors;
mod family_property;
mod molecular_property;
// mod molecule;  // Now in domain::entities
// mod molecule_family;  // Now in domain::entities
pub mod ports;
pub mod services;

// === Main Exports ===
pub use domain::*;
pub use errors::DomainError;

// === Legacy Exports (Phase 4: Remove) ===
pub use domain_stubs::{DomainStubs, InMemoryDomainRepository};
pub use family_property::FamilyProperty;
pub use molecular_property::MolecularProperty;
pub use molecule::Molecule as LegacyMolecule;
pub use molecule_family::MoleculeFamily as LegacyMoleculeFamily;
pub use services::{FamilyService, MoleculeService};

// === Convenience Re-exports ===
pub use ports::{
  AllDomainPorts, FamilyRepository, MoleculeReader, MoleculeStructure, MoleculeWriter, PropertyProvider, PropertyRepository,
  PropertyType, ProviderMolecule,
};

// === Application Layer Re-exports ===
pub use application::*;

/// Convenience prelude for common imports
pub mod prelude {
  pub use crate::application::{commands::*, queries::*};
  pub use crate::domain::{entities::*, events::*, ports::*, value_objects::*};
  pub use crate::DomainError;

  // Common external types
  pub use chrono::{DateTime, Utc};
  pub use serde_json::{json, Value as JsonValue};
  pub use uuid::Uuid;
}

/// Re-export owned properties from ports (new canonical location)
pub use ports::{OwnedFamilyProperty, OwnedMolecularProperty};
