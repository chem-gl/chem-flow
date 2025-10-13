//! # Application Layer
//!
//! The application layer orchestrates domain operations and implements use
//! cases. It follows CQRS (Command Query Responsibility Segregation) and
//! coordinates between the domain layer and external ports.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                Application Layer                        │
//! │                                                         │
//! │  📝 Commands                    📊 Queries              │
//! │  ├── CreateMoleculeFromSmiles   ├── GetMoleculeById     │
//! │  ├── UpdateMoleculeMetadata     ├── SearchBySmiles      │
//! │  └── DeleteMolecule             └── ListMolecules       │
//! │                                                         │
//! │  🎯 Use Cases (Application Services)                    │
//! │  ├── MoleculeCommandHandler                             │
//! │  ├── MoleculeQueryHandler                               │
//! │  └── PropertyCalculationService                         │
//! │                                                         │
//! │  ⬇️ Dependencies (Injected via Ports)                   │
//! │  ├── MoleculeRepository                                 │
//! │  ├── PropertyCalculator                                 │
//! │  └── EventPublisher                                     │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Design Principles
//!
//! - **Single Responsibility**: Each use case handles one business operation
//! - **Command/Query Separation**: Clear separation between reads and writes
//! - **Dependency Inversion**: Use cases depend on ports, not implementations
//! - **Event-Driven**: Publish domain events for integration
//!
//! ## Usage
//!
//! ```rust,no_run
//! use chem_domain::application::*;
//!
//! // Command (Write Operation)
//! let command = CreateMoleculeFromSmiles::new("CCO")
//!     .with_metadata(serde_json::json!({"name": "ethanol"}));
//!
//! // Query (Read Operation)
//! let query = GetMoleculeByInChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N")
//!     .with_properties();
//! ```

pub mod commands;
pub mod queries;
pub mod use_cases;

// Re-exports for convenience
pub use commands::*;
pub use queries::*;
pub use use_cases::{
  AddMoleculeToFamilyUseCase, CreateFamilyUseCase, CreateMoleculeUseCase, DeleteFamilyUseCase, DeleteMoleculeUseCase,
  GetFamilyPropertiesUseCase, GetFamilyUseCase, GetMolecularPropertiesUseCase, GetMoleculeUseCase, ListFamiliesUseCase,
  ListMoleculesUseCase, RemoveMoleculeFromFamilyUseCase, SaveFamilyPropertyUseCase, SaveMolecularPropertyUseCase,
};
