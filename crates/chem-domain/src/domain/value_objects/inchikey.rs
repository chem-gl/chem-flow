//! InChIKey Value Object
//!
//! Represents a standardized InChI Key with validation and type safety.
//! Prevents primitive obsession and encapsulates validation logic.
use crate::DomainError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
/// InChIKey value object with compile-time guarantees
///
/// Format: 14-character hash + 10-character hash + 1-character version
/// Example: LFQSCWFLJHTTHZ-UHFFFAOYSA-N
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InChIKey(String);
impl InChIKey {
  /// Create a new InChIKey with validation
  pub fn new(value: impl AsRef<str>) -> Result<Self, DomainError> {
    let normalized = value.as_ref().trim().to_uppercase();
    Self::validate(&normalized)?;
    Ok(Self(normalized))
  }
  /// Get the raw string value
  pub fn as_str(&self) -> &str {
    &self.0
  }
  /// Extract the hash portion of the InChIKey
  pub fn hash_portion(&self) -> &str {
    &self.0[0..14]
  }

  /// Extract the connectivity hash (first 14 characters)
  pub fn connectivity_hash(&self) -> &str {
    &self.0[0..14]
  }

  /// Extract the stereochemistry hash (characters 15-24)
  pub fn stereochemistry_hash(&self) -> &str {
    &self.0[15..25]
  }

  /// Extract the version indicator (last character)
  pub fn version(&self) -> char {
    self.0.chars().last().unwrap()
  }

  /// Get the length of the InChIKey string
  pub fn len(&self) -> usize {
    self.0.len()
  }

  /// Check if the InChIKey string is empty (should never be true for valid
  /// InChIKeys)
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// Validate InChIKey format
  fn validate(value: &str) -> Result<(), DomainError> {
    // Check length
    if value.len() != 27 {
      return Err(DomainError::validation("InChIKey", format!("Must be 27 characters, got {}", value.len())));
    }
    // Check format: 14-10-1
    if value.matches('-').count() != 2 {
      return Err(DomainError::validation("InChIKey", "Must contain exactly 2 hyphens".to_string()));
    }
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 3 {
      return Err(DomainError::validation("InChIKey", "Must have 3 parts separated by hyphens".to_string()));
    }
    // Validate part lengths
    if parts[0].len() != 14 {
      return Err(DomainError::validation("InChIKey", format!("First part must be 14 characters, got {}", parts[0].len())));
    }
    if parts[1].len() != 10 {
      return Err(DomainError::validation("InChIKey", format!("Second part must be 10 characters, got {}", parts[1].len())));
    }
    if parts[2].len() != 1 {
      return Err(DomainError::validation("InChIKey", format!("Third part must be 1 character, got {}", parts[2].len())));
    }
    // Validate character sets
    if !parts[0].chars().all(|c| c.is_ascii_uppercase()) {
      return Err(DomainError::validation("InChIKey", "First part must contain only uppercase letters".to_string()));
    }
    if !parts[1].chars().all(|c| c.is_ascii_uppercase()) {
      return Err(DomainError::validation("InChIKey", "Second part must contain only uppercase letters".to_string()));
    }
    if !parts[2].chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
      return Err(DomainError::validation("InChIKey",
                                         "Third part must contain only uppercase letters or digits".to_string()));
    }
    Ok(())
  }
}
impl fmt::Display for InChIKey {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}
impl FromStr for InChIKey {
  type Err = DomainError;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::new(s)
  }
}
impl TryFrom<String> for InChIKey {
  type Error = DomainError;
  fn try_from(value: String) -> Result<Self, Self::Error> {
    Self::new(value)
  }
}
impl TryFrom<&str> for InChIKey {
  type Error = DomainError;
  fn try_from(value: &str) -> Result<Self, Self::Error> {
    Self::new(value)
  }
}
impl AsRef<str> for InChIKey {
  fn as_ref(&self) -> &str {
    &self.0
  }
}

impl PartialEq<str> for InChIKey {
  fn eq(&self, other: &str) -> bool {
    self.0 == other
  }
}

impl PartialEq<&str> for InChIKey {
  fn eq(&self, other: &&str) -> bool {
    self.0 == *other
  }
}

impl PartialEq<String> for InChIKey {
  fn eq(&self, other: &String) -> bool {
    self.0 == *other
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn valid_inchikey_creation() {
    let key = InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N").unwrap();
    assert_eq!(key.as_str(), "LFQSCWFLJHTTHZ-UHFFFAOYSA-N");
    assert_eq!(key.connectivity_hash(), "LFQSCWFLJHTTHZ");
    assert_eq!(key.stereochemistry_hash(), "UHFFFAOYSA");
    assert_eq!(key.version(), 'N');
  }
  #[test]
  fn invalid_length_rejected() {
    let result = InChIKey::new("SHORT");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn invalid_format_rejected() {
    let result = InChIKey::new("LFQSCWFLJHTTHZUHFFFAOYSANN"); // No hyphens
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn wrong_part_lengths_rejected() {
    let result = InChIKey::new("SHORT-UHFFFAOYSA-N");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn lowercase_normalized() {
    let key = InChIKey::new("lfqscwfljhtthz-uhfffaoysa-n").unwrap();
    assert_eq!(key.as_str(), "LFQSCWFLJHTTHZ-UHFFFAOYSA-N");
  }
  #[test]
  fn from_str_works() {
    let key: InChIKey = "LFQSCWFLJHTTHZ-UHFFFAOYSA-N".parse().unwrap();
    assert_eq!(key.as_str(), "LFQSCWFLJHTTHZ-UHFFFAOYSA-N");
  }
}
