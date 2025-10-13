// ports/mod.rs
//! Ports (interfaces) que definen las capacidades que el dominio necesita del
//! mundo exterior. Estos traits serán implementados por adapters en otros
//! crates.
mod family_repository;
mod molecule_reader;
mod molecule_writer;
mod property_provider;
mod property_repository;
pub use family_repository::FamilyRepository;
pub use molecule_reader::MoleculeReader;
pub use molecule_writer::MoleculeWriter;
pub use property_provider::{MoleculeStructure, PropertyProvider, PropertyType, ProviderMolecule};
pub use property_repository::{OwnedFamilyProperty, OwnedMolecularProperty, PropertyRepository};
pub trait AllDomainPorts: MoleculeReader + MoleculeWriter + FamilyRepository + PropertyRepository + Send + Sync {}
impl<T> AllDomainPorts for T where T: MoleculeReader + MoleculeWriter + FamilyRepository + PropertyRepository + Send + Sync {}
