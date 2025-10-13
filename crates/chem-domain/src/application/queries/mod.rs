//! Query DTOs and Handlers
//!
//! Queries represent read operations in the CQRS pattern.
//! They encapsulate parameters for retrieving data without side effects.

use crate::domain::value_objects::{InChIKey, Smiles};
use crate::DomainError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Query to get a molecule by ID
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetMoleculeById {
  /// Molecule ID to retrieve
  pub molecule_id: Uuid,
  /// Include related data in response
  pub include_properties: bool,
}

impl GetMoleculeById {
  pub fn new(molecule_id: Uuid) -> Self {
    Self { molecule_id, include_properties: false }
  }

  pub fn with_properties(mut self) -> Self {
    self.include_properties = true;
    self
  }
}

/// Query to get a molecule by InChIKey
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetMoleculeByInChIKey {
  /// InChIKey to search for
  pub inchikey: String,
  /// Include related data in response
  pub include_properties: bool,
}

impl GetMoleculeByInChIKey {
  pub fn new(inchikey: impl Into<String>) -> Self {
    Self { inchikey: inchikey.into(), include_properties: false }
  }

  pub fn with_properties(mut self) -> Self {
    self.include_properties = true;
    self
  }

  /// Validate the query
  pub fn validate(&self) -> Result<(), DomainError> {
    InChIKey::new(&self.inchikey)?;
    Ok(())
  }
}

/// Query to search molecules by SMILES
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchMoleculesBySmiles {
  /// SMILES pattern to search for
  pub smiles: String,
  /// Exact match or similarity search
  pub exact_match: bool,
  /// Similarity threshold (0.0 to 1.0) for similarity search
  pub similarity_threshold: Option<f64>,
  /// Maximum number of results
  pub limit: Option<usize>,
}

impl SearchMoleculesBySmiles {
  pub fn exact(smiles: impl Into<String>) -> Self {
    Self { smiles: smiles.into(), exact_match: true, similarity_threshold: None, limit: None }
  }

  pub fn similar(smiles: impl Into<String>, threshold: f64) -> Self {
    Self { smiles: smiles.into(), exact_match: false, similarity_threshold: Some(threshold), limit: None }
  }

  pub fn with_limit(mut self, limit: usize) -> Self {
    self.limit = Some(limit);
    self
  }

  /// Validate the query
  pub fn validate(&self) -> Result<(), DomainError> {
    Smiles::new(&self.smiles)?;

    if let Some(threshold) = self.similarity_threshold {
      if !(0.0..=1.0).contains(&threshold) {
        return Err(DomainError::validation("SearchMoleculesBySmiles",
                                           "Similarity threshold must be between 0.0 and 1.0".to_string()));
      }
    }

    if let Some(limit) = self.limit {
      if limit == 0 {
        return Err(DomainError::validation("SearchMoleculesBySmiles", "Limit must be greater than 0".to_string()));
      }
    }

    Ok(())
  }
}

/// Query to list molecules with pagination
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListMolecules {
  /// Number of items to skip
  pub offset: usize,
  /// Maximum number of items to return
  pub limit: usize,
  /// Sort field
  pub sort_by: Option<String>,
  /// Sort direction
  pub sort_desc: bool,
  /// Filters to apply
  pub filters: HashMap<String, serde_json::Value>,
}

impl ListMolecules {
  pub fn new(offset: usize, limit: usize) -> Self {
    Self { offset, limit, sort_by: None, sort_desc: false, filters: HashMap::new() }
  }

  pub fn sort_by(mut self, field: impl Into<String>, descending: bool) -> Self {
    self.sort_by = Some(field.into());
    self.sort_desc = descending;
    self
  }

  pub fn with_filter(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
    self.filters.insert(key.into(), value);
    self
  }

  /// Validate the query
  pub fn validate(&self) -> Result<(), DomainError> {
    if self.limit == 0 {
      return Err(DomainError::validation("ListMolecules", "Limit must be greater than 0".to_string()));
    }

    if self.limit > 1000 {
      return Err(DomainError::validation("ListMolecules", "Limit cannot exceed 1000".to_string()));
    }

    Ok(())
  }
}

/// Query to search molecules by properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SearchMoleculesByProperties {
  /// Property filters (property_name -> range or exact value)
  pub property_filters: HashMap<String, PropertyFilter>,
  /// Maximum number of results
  pub limit: Option<usize>,
  /// Sort by property
  pub sort_by_property: Option<String>,
  /// Sort direction
  pub sort_desc: bool,
}

/// Property filter for search queries
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PropertyFilter {
  /// Exact value match
  Exact { value: f64 },
  /// Range filter
  Range { min: Option<f64>, max: Option<f64> },
  /// Greater than
  GreaterThan { value: f64 },
  /// Less than
  LessThan { value: f64 },
}

impl SearchMoleculesByProperties {
  pub fn new() -> Self {
    Self { property_filters: HashMap::new(), limit: None, sort_by_property: None, sort_desc: false }
  }

  pub fn with_exact_property(mut self, property: impl Into<String>, value: f64) -> Self {
    self.property_filters.insert(property.into(), PropertyFilter::Exact { value });
    self
  }

  pub fn with_property_range(mut self, property: impl Into<String>, min: Option<f64>, max: Option<f64>) -> Self {
    self.property_filters.insert(property.into(), PropertyFilter::Range { min, max });
    self
  }

  pub fn with_limit(mut self, limit: usize) -> Self {
    self.limit = Some(limit);
    self
  }

  pub fn sort_by_property(mut self, property: impl Into<String>, descending: bool) -> Self {
    self.sort_by_property = Some(property.into());
    self.sort_desc = descending;
    self
  }

  /// Validate the query
  pub fn validate(&self) -> Result<(), DomainError> {
    if self.property_filters.is_empty() {
      return Err(DomainError::validation("SearchMoleculesByProperties",
                                         "At least one property filter must be specified".to_string()));
    }

    for (property, filter) in &self.property_filters {
      if property.trim().is_empty() {
        return Err(DomainError::validation("SearchMoleculesByProperties", "Property name cannot be empty".to_string()));
      }

      if let PropertyFilter::Range { min, max } = filter {
        if let (Some(min_val), Some(max_val)) = (min, max) {
          if min_val >= max_val {
            return Err(DomainError::validation("SearchMoleculesByProperties",
                                               format!("Invalid range for {}: min must be less than max", property)));
          }
        }
      } // Other filters don't need validation
    }

    if let Some(limit) = self.limit {
      if limit == 0 || limit > 1000 {
        return Err(DomainError::validation("SearchMoleculesByProperties", "Limit must be between 1 and 1000".to_string()));
      }
    }

    Ok(())
  }
}

impl Default for SearchMoleculesByProperties {
  fn default() -> Self {
    Self::new()
  }
}

/// Query result wrapper with pagination info
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResult<T> {
  /// The actual data
  pub data: T,
  /// Total count (for pagination)
  pub total_count: Option<usize>,
  /// Pagination info
  pub pagination: Option<PaginationInfo>,
  /// Additional metadata
  pub metadata: serde_json::Value,
}

/// Pagination information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaginationInfo {
  /// Current offset
  pub offset: usize,
  /// Current limit
  pub limit: usize,
  /// Has more results
  pub has_more: bool,
  /// Next offset (if has_more is true)
  pub next_offset: Option<usize>,
}

impl<T> QueryResult<T> {
  pub fn new(data: T) -> Self {
    Self { data, total_count: None, pagination: None, metadata: serde_json::json!({}) }
  }

  pub fn with_total_count(mut self, total: usize) -> Self {
    self.total_count = Some(total);
    self
  }

  pub fn with_pagination(mut self, offset: usize, limit: usize, has_more: bool) -> Self {
    let next_offset = if has_more { Some(offset + limit) } else { None };
    self.pagination = Some(PaginationInfo { offset, limit, has_more, next_offset });
    self
  }

  pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
    self.metadata = metadata;
    self
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn get_molecule_by_inchikey_validation() {
    let valid_query = GetMoleculeByInChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N");
    assert!(valid_query.validate().is_ok());

    let invalid_query = GetMoleculeByInChIKey::new("INVALID");
    assert!(invalid_query.validate().is_err());
  }

  #[test]
  fn search_molecules_by_smiles_validation() {
    let valid_exact = SearchMoleculesBySmiles::exact("CCO");
    assert!(valid_exact.validate().is_ok());

    let valid_similar = SearchMoleculesBySmiles::similar("CCO", 0.8);
    assert!(valid_similar.validate().is_ok());

    let invalid_threshold = SearchMoleculesBySmiles::similar("CCO", 1.5);
    assert!(invalid_threshold.validate().is_err());

    let invalid_smiles = SearchMoleculesBySmiles::exact("");
    assert!(invalid_smiles.validate().is_err());
  }

  #[test]
  fn list_molecules_validation() {
    let valid_query = ListMolecules::new(0, 50);
    assert!(valid_query.validate().is_ok());

    let invalid_limit = ListMolecules::new(0, 0);
    assert!(invalid_limit.validate().is_err());

    let too_large_limit = ListMolecules::new(0, 2000);
    assert!(too_large_limit.validate().is_err());
  }

  #[test]
  fn search_by_properties_validation() {
    let valid_query = SearchMoleculesByProperties::new().with_property_range("molecular_weight", Some(100.0), Some(200.0));
    assert!(valid_query.validate().is_ok());

    let empty_query = SearchMoleculesByProperties::new();
    assert!(empty_query.validate().is_err());

    let invalid_range = SearchMoleculesByProperties::new().with_property_range("molecular_weight", Some(200.0), Some(100.0));
    assert!(invalid_range.validate().is_err());
  }

  #[test]
  fn query_result_creation() {
    let data = vec![1, 2, 3];
    let result = QueryResult::new(data.clone()).with_total_count(100).with_pagination(0, 10, true);

    assert_eq!(result.data, data);
    assert_eq!(result.total_count, Some(100));
    assert!(result.pagination.is_some());

    let pagination = result.pagination.unwrap();
    assert_eq!(pagination.offset, 0);
    assert_eq!(pagination.limit, 10);
    assert!(pagination.has_more);
    assert_eq!(pagination.next_offset, Some(10));
  }
}
