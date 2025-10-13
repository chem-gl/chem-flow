//! Molecular Formula Value Object
//!
//! Represents a molecular formula with parsing and validation capabilities.
use crate::DomainError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
/// Molecular formula value object with atomic composition
///
/// Represents a molecular formula (e.g., "C2H6O") with the ability
/// to parse and validate the atomic composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MolecularFormula {
  formula: String,
  composition: HashMap<String, u32>,
}
impl MolecularFormula {
  /// Create a new molecular formula with validation
  pub fn new(value: impl AsRef<str>) -> Result<Self, DomainError> {
    let normalized = value.as_ref().trim().to_string();
    let composition = Self::parse_composition(&normalized)?;
    Ok(Self { formula: normalized, composition })
  }
  /// Get the raw formula string
  pub fn as_str(&self) -> &str {
    &self.formula
  }
  /// Get the atomic composition
  pub fn composition(&self) -> &HashMap<String, u32> {
    &self.composition
  }
  /// Get the count of a specific element
  pub fn element_count(&self, element: &str) -> u32 {
    *self.composition.get(element).unwrap_or(&0)
  }
  /// Calculate the total number of atoms
  pub fn total_atoms(&self) -> u32 {
    self.composition.values().sum()
  }
  /// Get all elements present
  pub fn elements(&self) -> Vec<&String> {
    self.composition.keys().collect()
  }
  /// Check if contains a specific element
  pub fn contains_element(&self, element: &str) -> bool {
    self.composition.contains_key(element)
  }
  /// Calculate molecular weight (rough estimate with common atomic weights)
  pub fn molecular_weight_estimate(&self) -> f64 {
    let atomic_weights = [("H", 1.008),
                          ("C", 12.011),
                          ("N", 14.007),
                          ("O", 15.999),
                          ("F", 18.998),
                          ("P", 30.974),
                          ("S", 32.065),
                          ("Cl", 35.453),
                          ("Br", 79.904),
                          ("I", 126.904)];
    let weights: HashMap<&str, f64> = atomic_weights.iter().cloned().collect();
    self.composition
        .iter()
        .map(|(element, count)| {
          let weight = weights.get(element.as_str()).unwrap_or(&12.0); // Default to carbon
          weight * (*count as f64)
        })
        .sum()
  }
  /// Parse atomic composition from formula string
  fn parse_composition(formula: &str) -> Result<HashMap<String, u32>, DomainError> {
    if formula.is_empty() {
      return Err(DomainError::validation("MolecularFormula", "Cannot be empty".to_string()));
    }
    let mut composition = HashMap::new();
    let mut chars = formula.chars().peekable();
    while let Some(c) = chars.next() {
      if !c.is_ascii_uppercase() {
        return Err(DomainError::validation("MolecularFormula",
                                           format!("Invalid character '{}' - elements must start with uppercase", c)));
      }
      let mut element = String::new();
      element.push(c);
      // Read lowercase letters for multi-character elements
      while let Some(&next_c) = chars.peek() {
        if next_c.is_ascii_lowercase() {
          element.push(chars.next().unwrap());
        } else {
          break;
        }
      }
      // Read the count
      let mut count_str = String::new();
      while let Some(&next_c) = chars.peek() {
        if next_c.is_ascii_digit() {
          count_str.push(chars.next().unwrap());
        } else {
          break;
        }
      }
      let count = if count_str.is_empty() {
        1
      } else {
        count_str.parse::<u32>().map_err(|_| {
                                   DomainError::validation("MolecularFormula",
                                                           format!("Invalid count '{}' for element '{}'",
                                                                   count_str, element))
                                 })?
      };
      if count == 0 {
        return Err(DomainError::validation("MolecularFormula", format!("Element '{}' cannot have zero count", element)));
      }
      *composition.entry(element).or_insert(0) += count;
    }
    if composition.is_empty() {
      return Err(DomainError::validation("MolecularFormula", "No valid elements found".to_string()));
    }
    Ok(composition)
  }
}
impl fmt::Display for MolecularFormula {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.formula)
  }
}
impl FromStr for MolecularFormula {
  type Err = DomainError;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Self::new(s)
  }
}
impl TryFrom<String> for MolecularFormula {
  type Error = DomainError;
  fn try_from(value: String) -> Result<Self, Self::Error> {
    Self::new(value)
  }
}
impl TryFrom<&str> for MolecularFormula {
  type Error = DomainError;
  fn try_from(value: &str) -> Result<Self, Self::Error> {
    Self::new(value)
  }
}
impl AsRef<str> for MolecularFormula {
  fn as_ref(&self) -> &str {
    &self.formula
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn valid_formula_creation() {
    let formula = MolecularFormula::new("C2H6O").unwrap();
    assert_eq!(formula.as_str(), "C2H6O");
    assert_eq!(formula.element_count("C"), 2);
    assert_eq!(formula.element_count("H"), 6);
    assert_eq!(formula.element_count("O"), 1);
    assert_eq!(formula.total_atoms(), 9);
  }
  #[test]
  fn single_character_elements() {
    let formula = MolecularFormula::new("H2O").unwrap();
    assert_eq!(formula.element_count("H"), 2);
    assert_eq!(formula.element_count("O"), 1);
  }
  #[test]
  fn multi_character_elements() {
    let formula = MolecularFormula::new("CaCl2").unwrap();
    assert_eq!(formula.element_count("Ca"), 1);
    assert_eq!(formula.element_count("Cl"), 2);
  }
  #[test]
  fn no_count_defaults_to_one() {
    let formula = MolecularFormula::new("CO").unwrap();
    assert_eq!(formula.element_count("C"), 1);
    assert_eq!(formula.element_count("O"), 1);
  }
  #[test]
  fn empty_formula_rejected() {
    let result = MolecularFormula::new("");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn invalid_start_character_rejected() {
    let result = MolecularFormula::new("2H2O");
    assert!(matches!(result, Err(DomainError::ValidationError { .. })));
  }
  #[test]
  fn molecular_weight_calculation() {
    let ethanol = MolecularFormula::new("C2H6O").unwrap();
    let weight = ethanol.molecular_weight_estimate();
    // C: 12.011 * 2 = 24.022
    // H: 1.008 * 6 = 6.048
    // O: 15.999 * 1 = 15.999
    // Total ≈ 46.069
    assert!((weight - 46.069).abs() < 0.1);
  }
  #[test]
  fn contains_element() {
    let formula = MolecularFormula::new("C2H6O").unwrap();
    assert!(formula.contains_element("C"));
    assert!(formula.contains_element("H"));
    assert!(formula.contains_element("O"));
    assert!(!formula.contains_element("N"));
  }
  #[test]
  fn from_str_works() {
    let formula: MolecularFormula = "C2H6O".parse().unwrap();
    assert_eq!(formula.element_count("C"), 2);
  }
}
