// lib.rs
//! Chem Domain - Núcleo de dominio puro
//!
//! Este crate contiene:
//! - Entidades del dominio (Molecule, MoleculeFamily, Properties)
//! - Ports (traits que definen interfaces con el mundo exterior)
//! - Servicios de dominio (lógica de negocio)
//! - Errores del dominio
//!
//! **Principio de Dependencias**: Este crate NO debe depender de otros crates
//! de la aplicación (chem-persistence, chem-providers, etc.). Solo dependencias
//! estándar de Rust y librerías utilitarias (serde, uuid, etc.).

// === Modules ===
pub mod application; // ✅ Phase 3: Application Layer (Use Cases)
mod domain_stubs;
mod errors;
mod family_property;
mod molecular_property;
mod molecule;
mod molecule_family;

// === Ports (interfaces con el exterior) ===
pub mod ports;

// === Services (lógica de negocio del dominio) ===
pub mod services;

// === Exports Públicos ===

// Legacy repository - solo para compatibilidad
pub use domain_stubs::{DomainStubs, InMemoryDomainRepository};

// Exports principales
pub use errors::DomainError;
pub use family_property::FamilyProperty;
pub use molecular_property::MolecularProperty;
pub use molecule::Molecule;
pub use molecule_family::MoleculeFamily;

// Re-export de owned properties desde ports (nueva ubicación canónica)
pub use ports::{OwnedFamilyProperty, OwnedMolecularProperty};

// Re-export de ports para facilitar uso
pub use ports::{
  AllDomainPorts, FamilyRepository, MoleculeReader, MoleculeStructure, MoleculeWriter, PropertyProvider, PropertyRepository,
  PropertyType, ProviderMolecule,
};

// Re-export de services para facilitar uso
pub use services::{FamilyService, MoleculeService};
