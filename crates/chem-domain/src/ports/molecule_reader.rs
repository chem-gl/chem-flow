// ports/molecule_reader.rs
//! Port para operaciones de lectura de moléculas
use crate::{DomainError, Molecule};
/// Port para leer moléculas (CQRS pattern)
pub trait MoleculeReader: Send + Sync {
  /// Obtiene una molécula por su InChIKey
  fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError>;
  /// Lista todas las moléculas
  fn list_molecules(&self) -> Result<Vec<Molecule>, DomainError>;
  /// Busca moléculas por SMILES
  fn find_by_smiles(&self, smiles: &str) -> Result<Vec<Molecule>, DomainError>;
}
