mod domain_repository;
mod domain_stubs;
mod errors;
mod family_property;
mod molecular_property;
mod molecule;
mod molecule_family;

pub use domain_repository::{DomainRepository, InMemoryDomainRepository};
pub use errors::DomainError;
pub use family_property::FamilyProperty;
pub use molecular_property::MolecularProperty;
pub use molecule::Molecule;
pub use molecule_family::MoleculeFamily;
// Re-export owned DTOs so external persistence crates can reference them
pub use domain_repository::{OwnedFamilyProperty, OwnedMolecularProperty};
pub use domain_stubs::DomainStubs;
