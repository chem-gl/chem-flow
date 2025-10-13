//! InChI Value Object
//!
//! Represents an InChI (International Chemical Identifier) string
//! with validation and parsing capabilities.
use crate::DomainError;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
/// InChI value object with validation
///
/// Represents a standardized InChI string that uniquely identifies
/// a chemical structure.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InChI(String);
impl InChI {
  /// Create a new InChI with validation
  pub fn new(value: impl AsRef<str>) -> Result<Self, DomainError> {
    let normalized = value.as_ref().trim().to_string();
    Self::validate(&normalized)?;
    Ok(Self(normalized))
  }
  /// Get the raw string value
  pub fn as_str(&self) -> &str {
    &self.0
  }
  /// Get the version of the InChI standard
  pub fn version(&self) -> &str {
    if self.0.starts_with("InChI=1S/") {
      "1S"
    } else if self.0.starts_with("InChI=1/") {
      "1"
    } else {
      "unknown"
    }
  }
  /// Extract the molecular formula if present
  pub fn molecular_formula(&self) -> Option<&str> {
    if let Some(start) = self.0.find("/") {
      let remaining = &self.0[start + 1..];
      if let Some(end) = remaining.find("/") {
        Some(&remaining[..end])
      } else {
        Some(remaining)
      }
    } else {
      None
    }
  }
  /// Check if the InChI represents a multi-component structure
  pub fn is_multi_component(&self) -> bool {
    self.0.contains('.')
  }

  /// Check if the InChI contains stereochemistry information
  pub fn has_stereochemistry(&self) -> bool {
    self.0.contains("/t") || self.0.contains("/m") || self.0.contains("/s")
  }

  /// Check if the InChI string is empty
  pub fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// Get the length of the InChI string
  pub fn len(&self) -> usize {
    self.0.len()
  }
  /// Validate InChI format
  fn validate(value: &str) -> Result<(), DomainError> {
    if value.is_empty() {
      return Err(DomainError::validation("InChI", "Cannot be empty".to_string()));
    }
    if !value.starts_with("InChI=") {
      return Err(DomainError::validation("InChI", "Must start with 'InChI='".to_string()));
    }
    if value.len() < 9 {
      // Minimum: "InChI=1S/"
      return Err(DomainError::validation("InChI", "Too short to be a valid InChI".to_string()));
    }
    if value.len() > 32768 {
      // Reasonable maximum
      return Err(DomainError::validation("InChI", format!("Too long: {} characters, maximum 32768", value.len())));
    }
    // Check for valid version
    if !value.starts_with("InChI=1S/") && !value.starts_with("InChI=1/") {
      return Err(DomainError::validation("InChI", "Unsupported InChI version".to_string()));
    }
    // Basic character validation
    if !value.chars().all(|c| {
                       c.is_ascii_alphanumeric()
                       || matches!(c, '=' | '/' | '\\' | '(' | ')' | '[' | ']' | '+' | '-' | ',' | ';' | ':')
                     })
    {
      return Err(DomainError::validation("InChI", "Contains invalid characters".to_string()));
    }
    Ok(())
  }
}
impl fmt::Display for InChI {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.0)
  }
}
impl FromStr for InChI {
  type Err = DomainError;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::new(s)
  }
}
impl TryFrom<String> for InChI {
  type Error = DomainError;
  fn try_from(value: String) -> Result<Self, Self::Error> {
    Self::new(value)
  }
}
impl TryFrom<&str> for InChI {
  type Error = DomainError;
  fn try_from(value: &str) -> Result<Self, Self::Error> {
    Self::new(value)
  }
}
impl AsRef<str> for InChI {
  fn as_ref(&self) -> &str {
    &self.0
  }
}

impl PartialEq<str> for InChI {
  fn eq(&self, other: &str) -> bool {
    self.0 == other
  }
}

impl PartialEq<&str> for InChI {
  fn eq(&self, other: &&str) -> bool {
    self.0 == *other
  }
}

impl PartialEq<String> for InChI {
  fn eq(&self, other: &String) -> bool {
    self.0 == *other
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn valid_inchi_creation() {
    let inchi = InChI::new("InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3").unwrap();
    assert_eq!(inchi.version(), "1S");
    assert_eq!(inchi.molecular_formula(), Some("C2H6O"));
    assert!(!inchi.has_stereochemistry());
  }
  #[test]
  fn stereochemistry_detection() {
    let inchi = InChI::new("InChI=1S/C4H8O2/c1-3-4(2)5-6/h4H,3H2,1-2H3/t4-/m0/s1").unwrap();
    assert!(inchi.has_stereochemistry());
  }
  #[test]
  fn empty_inchi_rejected() {
    let result = InChI::new("");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn invalid_prefix_rejected() {
    let result = InChI::new("NotAnInChI");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn too_short_rejected() {
    let result = InChI::new("InChI=");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn whitespace_trimmed() {
    let inchi = InChI::new("  InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3  ").unwrap();
    assert_eq!(inchi.as_str(), "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3");
  }
  #[test]
  fn from_str_works() {
    let inchi: InChI = "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3".parse().unwrap();
    assert_eq!(inchi.version(), "1S");
  }
}
