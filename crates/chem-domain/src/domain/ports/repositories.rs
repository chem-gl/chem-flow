//! Repository Ports
//!
//! Persistence contracts following CQRS pattern and Interface Segregation
//! Principle. Each interface is focused and cohesive, allowing implementations
//! to be specialized.

use crate::domain::entities::Molecule;
use crate::domain::value_objects::{InChIKey, Smiles};
use crate::DomainError;
use async_trait::async_trait;
use uuid::Uuid;

/// Query-side repository for molecule reads
///
/// Separated from writes to allow different optimization strategies
/// and to follow the Command/Query Responsibility Segregation pattern.
#[async_trait]
pub trait MoleculeQuery: Send + Sync {
  /// Find molecule by its unique identifier
  async fn find_by_id(&self, id: &Uuid) -> Result<Option<Molecule>, DomainError>;

  /// Find molecule by InChIKey (chemical identifier)
  async fn find_by_inchikey(&self, inchikey: &InChIKey) -> Result<Option<Molecule>, DomainError>;

  /// Search molecules by SMILES pattern
  async fn find_by_smiles(&self, smiles: &Smiles) -> Result<Vec<Molecule>, DomainError>;

  /// List all molecules (paginated)
  async fn list_molecules(&self, offset: usize, limit: usize) -> Result<Vec<Molecule>, DomainError>;

  /// Count total molecules
  async fn count_molecules(&self) -> Result<usize, DomainError>;

  /// Check if molecule exists by InChIKey
  async fn exists_by_inchikey(&self, inchikey: &InChIKey) -> Result<bool, DomainError>;
}

/// Command-side repository for molecule writes
///
/// Focused on persistence operations that modify state.
/// Separated from queries to enable different scaling strategies.
#[async_trait]
pub trait MoleculeCommand: Send + Sync {
  /// Save a new molecule
  async fn save(&self, molecule: Molecule) -> Result<Uuid, DomainError>;

  /// Update an existing molecule
  async fn update(&self, molecule: Molecule) -> Result<(), DomainError>;

  /// Delete molecule by ID
  async fn delete_by_id(&self, id: &Uuid) -> Result<(), DomainError>;

  /// Delete molecule by InChIKey
  async fn delete_by_inchikey(&self, inchikey: &InChIKey) -> Result<(), DomainError>;

  /// Batch save multiple molecules
  async fn save_batch(&self, molecules: Vec<Molecule>) -> Result<Vec<Uuid>, DomainError>;
}

/// Combined repository interface for convenience
///
/// Some implementations may want to provide both read and write capabilities
/// through a single service. This trait combines both for convenience.
pub trait MoleculeRepository: MoleculeQuery + MoleculeCommand + Send + Sync {}

// Blanket implementation for any type that implements both traits
impl<T> MoleculeRepository for T where T: MoleculeQuery + MoleculeCommand + Send + Sync {}

/// Specialized query interface for complex molecule searches
///
/// Advanced search capabilities that might be implemented differently
/// from basic queries (e.g., using search engines or specialized databases).
#[async_trait]
pub trait MoleculeSearch: Send + Sync {
  /// Full-text search across molecule metadata
  async fn search_by_text(&self, query: &str) -> Result<Vec<Molecule>, DomainError>;

  /// Search by molecular properties
  async fn search_by_properties(&self,
                                properties: &std::collections::HashMap<String, f64>)
                                -> Result<Vec<Molecule>, DomainError>;

  /// Search by molecular weight range
  async fn search_by_molecular_weight_range(&self, min_weight: f64, max_weight: f64) -> Result<Vec<Molecule>, DomainError>;

  /// Search similar molecules (structural similarity)
  async fn search_similar(&self, reference: &Molecule, threshold: f64) -> Result<Vec<Molecule>, DomainError>;
}

/// Transaction management for atomic operations
///
/// Enables atomic operations across multiple repository calls.
/// Useful for complex business operations that must be consistent.
#[async_trait]
pub trait TransactionManager: Send + Sync {
  type Transaction: Send + Sync;

  /// Begin a new transaction
  async fn begin(&self) -> Result<Self::Transaction, DomainError>;

  /// Commit the transaction
  async fn commit(&self, transaction: Self::Transaction) -> Result<(), DomainError>;

  /// Rollback the transaction
  async fn rollback(&self, transaction: Self::Transaction) -> Result<(), DomainError>;
}

/// Repository with transaction support
///
/// Extends the basic repository with transaction capabilities.
/// Implementations can coordinate multiple operations atomically.
#[async_trait]
pub trait TransactionalMoleculeRepository: MoleculeRepository + Send + Sync {
  type Transaction: Send + Sync;

  /// Execute operations within a transaction
  async fn with_transaction<F, R>(&self, f: F) -> Result<R, DomainError>
    where F: FnOnce(&Self::Transaction) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<R, DomainError>> + Send>>
            + Send,
          R: Send;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::value_objects::*;
  use std::sync::Arc;

  // Mock implementation for testing
  struct MockMoleculeQuery;

  #[async_trait]
  impl MoleculeQuery for MockMoleculeQuery {
    async fn find_by_id(&self, _id: &Uuid) -> Result<Option<Molecule>, DomainError> {
      Ok(None)
    }

    async fn find_by_inchikey(&self, _inchikey: &InChIKey) -> Result<Option<Molecule>, DomainError> {
      Ok(None)
    }

    async fn find_by_smiles(&self, _smiles: &Smiles) -> Result<Vec<Molecule>, DomainError> {
      Ok(vec![])
    }

    async fn list_molecules(&self, _offset: usize, _limit: usize) -> Result<Vec<Molecule>, DomainError> {
      Ok(vec![])
    }

    async fn count_molecules(&self) -> Result<usize, DomainError> {
      Ok(0)
    }

    async fn exists_by_inchikey(&self, _inchikey: &InChIKey) -> Result<bool, DomainError> {
      Ok(false)
    }
  }

  struct MockMoleculeCommand;

  #[async_trait]
  impl MoleculeCommand for MockMoleculeCommand {
    async fn save(&self, _molecule: Molecule) -> Result<Uuid, DomainError> {
      Ok(Uuid::new_v4())
    }

    async fn update(&self, _molecule: Molecule) -> Result<(), DomainError> {
      Ok(())
    }

    async fn delete_by_id(&self, _id: &Uuid) -> Result<(), DomainError> {
      Ok(())
    }

    async fn delete_by_inchikey(&self, _inchikey: &InChIKey) -> Result<(), DomainError> {
      Ok(())
    }

    async fn save_batch(&self, molecules: Vec<Molecule>) -> Result<Vec<Uuid>, DomainError> {
      Ok(molecules.into_iter().map(|_| Uuid::new_v4()).collect())
    }
  }

  #[tokio::test]
  async fn query_interface_segregation() {
    let query: Arc<dyn MoleculeQuery> = Arc::new(MockMoleculeQuery);
    let count = query.count_molecules().await.unwrap();
    assert_eq!(count, 0);
  }

  #[tokio::test]
  async fn command_interface_segregation() {
    let command: Arc<dyn MoleculeCommand> = Arc::new(MockMoleculeCommand);
    let inchikey = InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N").unwrap();
    let result = command.delete_by_inchikey(&inchikey).await;
    assert!(result.is_ok());
  }
}
