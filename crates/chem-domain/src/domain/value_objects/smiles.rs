//! SMILES Value Object
//!
//! Represents a SMILES (Simplified Molecular Input Line Entry System) string
//! with validation and normalization capabilities.
use crate::DomainError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
/// SMILES value object with validation
///
/// Encapsulates SMILES string representation with basic validation.
/// Advanced chemical validation should be performed by PropertyProvider.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Smiles(String);
impl Smiles {
  /// Create a new SMILES with basic validation
  pub fn new(value: impl AsRef<str>) -> Result<Self, DomainError> {
    let normalized = value.as_ref().trim().to_string();
    Self::validate(&normalized)?;
    Ok(Self(normalized))
  }
  /// Get the raw string value
  pub fn as_str(&self) -> &str {
    &self.0
  }
  /// Check if this is likely an aromatic compound
  pub fn is_aromatic(&self) -> bool {
    self.0.chars().any(|c| c.is_ascii_lowercase() && c.is_alphabetic())
  }
  /// Count the number of atoms (rough estimate)
  pub fn atom_count_estimate(&self) -> usize {
    self.0.chars().filter(|c| c.is_ascii_alphabetic()).count()
  }
  /// Check if contains rings
  pub fn has_rings(&self) -> bool {
    self.0.chars().any(|c| c.is_ascii_digit())
  }

  /// Check if the SMILES string is empty
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// Get the length of the SMILES string
  pub fn len(&self) -> usize {
    self.0.len()
  }
  /// Basic validation for SMILES format
  fn validate(value: &str) -> Result<(), DomainError> {
    if value.is_empty() {
      return Err(DomainError::validation("SMILES", "Cannot be empty".to_string()));
    }
    if value.len() > 1000 {
      return Err(DomainError::validation("SMILES", format!("Too long: {} characters, maximum 1000", value.len())));
    }
    // Basic character validation
    if !value.chars().all(|c| {
                       c.is_ascii_alphanumeric()
                       || matches!(c, '(' | ')' | '[' | ']' | '=' | '#' | '\\' | '/' | '+' | '-' | '@' | '.')
                     })
    {
      return Err(DomainError::validation("SMILES", "Contains invalid characters".to_string()));
    }
    // Check balanced parentheses
    let mut paren_count = 0;
    let mut bracket_count = 0;
    for c in value.chars() {
      match c {
        '(' => paren_count += 1,
        ')' => {
          paren_count -= 1;
          if paren_count < 0 {
            return Err(DomainError::validation("SMILES", "Unbalanced parentheses".to_string()));
          }
        }
        '[' => bracket_count += 1,
        ']' => {
          bracket_count -= 1;
          if bracket_count < 0 {
            return Err(DomainError::validation("SMILES", "Unbalanced brackets".to_string()));
          }
        }
        _ => {}
      }
    }
    if paren_count != 0 {
      return Err(DomainError::validation("SMILES", "Unbalanced parentheses".to_string()));
    }
    if bracket_count != 0 {
      return Err(DomainError::validation("SMILES", "Unbalanced brackets".to_string()));
    }
    Ok(())
  }
}
impl fmt::Display for Smiles {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}
impl FromStr for Smiles {
  type Err = DomainError;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::new(s)
  }
}
impl TryFrom<String> for Smiles {
  type Error = DomainError;
  fn try_from(value: String) -> Result<Self, Self::Error> {
    Self::new(value)
  }
}
impl TryFrom<&str> for Smiles {
  type Error = DomainError;
  fn try_from(value: &str) -> Result<Self, Self::Error> {
    Self::new(value)
  }
}
impl AsRef<str> for Smiles {
  fn as_ref(&self) -> &str {
    &self.0
  }
}

impl PartialEq<str> for Smiles {
  fn eq(&self, other: &str) -> bool {
    self.0 == other
  }
}

impl PartialEq<&str> for Smiles {
  fn eq(&self, other: &&str) -> bool {
    self.0 == *other
  }
}

impl PartialEq<String> for Smiles {
  fn eq(&self, other: &String) -> bool {
    self.0 == *other
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn valid_smiles_creation() {
    let smiles = Smiles::new("CCO").unwrap();
    assert_eq!(smiles.as_str(), "CCO");
    assert!(!smiles.is_aromatic());
    assert!(!smiles.has_rings());
  }
  #[test]
  fn aromatic_detection() {
    let benzene = Smiles::new("c1ccccc1").unwrap();
    assert!(benzene.is_aromatic());
    assert!(benzene.has_rings());
  }
  #[test]
  fn empty_smiles_rejected() {
    let result = Smiles::new("");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn unbalanced_parentheses_rejected() {
    let result = Smiles::new("C(C");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn unbalanced_brackets_rejected() {
    let result = Smiles::new("C[CH2");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn whitespace_trimmed() {
    let smiles = Smiles::new("  CCO  ").unwrap();
    assert_eq!(smiles.as_str(), "CCO");
  }
  #[test]
  fn atom_count_estimate() {
    let ethanol = Smiles::new("CCO").unwrap();
    assert_eq!(ethanol.atom_count_estimate(), 3);
  }
  #[test]
  fn from_str_works() {
    let smiles: Smiles = "CCO".parse().unwrap();
    assert_eq!(smiles.as_str(), "CCO");
  }
}
