// ports/molecule_writer.rs
//! Port para operaciones de escritura de moléculas
use crate::{DomainError, Molecule};
/// Port para escribir moléculas (CQRS pattern)
pub trait MoleculeWriter: Send + Sync {
  /// Guarda una molécula, retorna el InChIKey
  fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError>;
  /// Elimina una molécula por su InChIKey
  fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError>;
}
