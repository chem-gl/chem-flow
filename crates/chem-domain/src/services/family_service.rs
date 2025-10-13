use crate::ports::{FamilyRepository, MoleculeReader};
use crate::{DomainError, Molecule, MoleculeFamily};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;
/// Service for molecular family-related business operations
///
/// Provides business logic for family management, aggregation operations,
/// and complex family queries while maintaining domain purity.
pub struct FamilyService<R>
  where R: FamilyRepository + MoleculeReader
{
  repository: R,
}
impl<R> FamilyService<R> where R: FamilyRepository + MoleculeReader
{
  /// Create a new family service with the given repository
  pub fn new(repository: R) -> Self {
    Self { repository }
  }
  /// Create a family from a collection of molecule InChIKeys
  ///
  /// Validates that all molecules exist before creating the family.
  pub fn create_family_from_molecule_ids(&self,
                                         molecule_inchikeys: Vec<String>,
                                         metadata: Value)
                                         -> Result<Uuid, DomainError> {
    // Retrieve all molecules to ensure they exist
    let mut molecules = Vec::new();
    for inchikey in molecule_inchikeys {
      match self.repository.get_molecule(&inchikey)? {
        Some(molecule) => molecules.push(molecule),
        None => {
          return Err(DomainError::not_found("molecule", format!("Molecule with InChIKey {} not found", inchikey)));
        }
      }
    }
    // Create and save family
    let family = MoleculeFamily::new(molecules, metadata)?;
    self.repository.save_family(family)
  }
  /// Create a family from molecules directly
  pub fn create_family_from_molecules(&self, molecules: Vec<Molecule>, metadata: Value) -> Result<Uuid, DomainError> {
    let family = MoleculeFamily::new(molecules, metadata)?;
    self.repository.save_family(family)
  }
  /// Retrieve a family by ID
  pub fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError> {
    self.repository.get_family(id)
  }
  /// Calculate aggregate properties for a family
  ///
  /// Computes statistical measures across all molecules in the family.
  pub fn calculate_family_statistics(&self, family_id: &Uuid) -> Result<FamilyStatistics, DomainError> {
    let family = match self.repository.get_family(family_id)? {
      Some(f) => f,
      None => return Err(DomainError::not_found("family", format!("Family {} not found", family_id))),
    };
    let molecules = family.molecules();
    let count = molecules.len();
    if count == 0 {
      return Ok(FamilyStatistics::empty());
    }
    // Extract numerical properties from metadata for aggregation
    let mut properties: HashMap<String, Vec<f64>> = HashMap::new();
    for molecule in molecules {
      let metadata = molecule.metadata();
      if let Some(obj) = metadata.as_object() {
        for (key, value) in obj {
          if let Some(num) = value.as_f64() {
            properties.entry(key.clone()).or_default().push(num);
          }
        }
      }
    }
    // Calculate statistics for each property
    let mut property_stats = HashMap::new();
    for (property, values) in properties {
      if !values.is_empty() {
        let sum: f64 = values.iter().sum();
        let mean = sum / values.len() as f64;
        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min = *sorted_values.first().unwrap();
        let max = *sorted_values.last().unwrap();
        let median = if sorted_values.len() % 2 == 0 {
          let mid = sorted_values.len() / 2;
          (sorted_values[mid - 1] + sorted_values[mid]) / 2.0
        } else {
          sorted_values[sorted_values.len() / 2]
        };
        property_stats.insert(property,
                              PropertyStatistics { count: values.len(), sum, mean, median, min, max });
      }
    }
    Ok(FamilyStatistics { molecule_count: count, property_statistics: property_stats })
  }
  /// Find families containing a specific molecule
  pub fn find_families_containing_molecule(&self, molecule_inchikey: &str) -> Result<Vec<MoleculeFamily>, DomainError> {
    let molecule = match self.repository.get_molecule(molecule_inchikey)? {
      Some(m) => m,
      None => return Ok(Vec::new()),
    };
    let all_families = self.repository.list_families()?;
    Ok(all_families.into_iter()
                   .filter(|family| family.molecules().iter().any(|m| m.inchikey() == molecule.inchikey()))
                   .collect())
  }
  /// Merge multiple families into a new family
  ///
  /// Combines molecules from multiple families, removing duplicates.
  pub fn merge_families(&self, family_ids: Vec<Uuid>, new_metadata: Value) -> Result<Uuid, DomainError> {
    let mut all_molecules = Vec::new();
    let mut seen_inchikeys = std::collections::HashSet::new();
    for family_id in family_ids {
      let family = match self.repository.get_family(&family_id)? {
        Some(f) => f,
        None => continue, // Skip missing families
      };
      for molecule in family.molecules() {
        if seen_inchikeys.insert(molecule.inchikey().to_string()) {
          all_molecules.push(molecule.clone());
        }
      }
    }
    if all_molecules.is_empty() {
      return Err(DomainError::validation("family", "Cannot create empty family from merge"));
    }
    let merged_family = MoleculeFamily::new(all_molecules, new_metadata)?;
    self.repository.save_family(merged_family)
  }
  /// Get family diversity metrics
  ///
  /// Calculates measures of structural diversity within the family.
  pub fn calculate_family_diversity(&self, family_id: &Uuid) -> Result<DiversityMetrics, DomainError> {
    let family = match self.repository.get_family(family_id)? {
      Some(f) => f,
      None => return Err(DomainError::not_found("family", format!("Family {} not found", family_id))),
    };
    let molecules = family.molecules();
    let total_count = molecules.len();
    if total_count == 0 {
      return Ok(DiversityMetrics::empty());
    }
    // Calculate unique SMILES patterns
    let unique_smiles: std::collections::HashSet<_> = molecules.iter().map(|m| m.smiles()).collect();
    // Calculate unique InChI patterns
    let unique_inchi: std::collections::HashSet<_> = molecules.iter().map(|m| m.inchi()).collect();
    // Diversity ratio (unique structures / total structures)
    let smiles_diversity = unique_smiles.len() as f64 / total_count as f64;
    let inchi_diversity = unique_inchi.len() as f64 / total_count as f64;
    Ok(DiversityMetrics { total_molecules: total_count,
                          unique_smiles_count: unique_smiles.len(),
                          unique_inchi_count: unique_inchi.len(),
                          smiles_diversity_ratio: smiles_diversity,
                          inchi_diversity_ratio: inchi_diversity })
  }
}
/// Statistical aggregation results for a family
#[derive(Debug, Clone)]
pub struct FamilyStatistics {
  pub molecule_count: usize,
  pub property_statistics: HashMap<String, PropertyStatistics>,
}
impl FamilyStatistics {
  pub fn empty() -> Self {
    Self { molecule_count: 0, property_statistics: HashMap::new() }
  }
}
/// Statistics for a specific property across a family
#[derive(Debug, Clone)]
pub struct PropertyStatistics {
  pub count: usize,
  pub sum: f64,
  pub mean: f64,
  pub median: f64,
  pub min: f64,
  pub max: f64,
}
/// Diversity metrics for a molecular family
#[derive(Debug, Clone)]
pub struct DiversityMetrics {
  pub total_molecules: usize,
  pub unique_smiles_count: usize,
  pub unique_inchi_count: usize,
  pub smiles_diversity_ratio: f64,
  pub inchi_diversity_ratio: f64,
}
impl DiversityMetrics {
  pub fn empty() -> Self {
    Self { total_molecules: 0,
           unique_smiles_count: 0,
           unique_inchi_count: 0,
           smiles_diversity_ratio: 0.0,
           inchi_diversity_ratio: 0.0 }
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  use crate::InMemoryDomainRepository;
  use serde_json::json;
  #[test]
  fn test_create_family_from_molecules() {
    let repo = InMemoryDomainRepository::new();
    let service = FamilyService::new(repo);
    let mol1 = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                    "CCO",
                                    "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                    json!({"weight": 46.07})).unwrap();
    let mol2 = Molecule::from_parts("OTMSDBZUPAUEDD-UHFFFAOYSA-N",
                                    "CC",
                                    "InChI=1S/C2H6/c1-2/h1-2H3",
                                    json!({"weight": 30.07})).unwrap();
    let family_id = service.create_family_from_molecules(vec![mol1, mol2], json!({"name": "test_family"})).unwrap();
    let family = service.get_family(&family_id).unwrap().unwrap();
    assert_eq!(family.molecules().len(), 2);
  }
  #[test]
  fn test_calculate_family_statistics() {
    let repo = InMemoryDomainRepository::new();
    let service = FamilyService::new(repo);
    let mol1 = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                    "CCO",
                                    "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                    json!({"weight": 46.07, "logP": -0.31})).unwrap();
    let mol2 = Molecule::from_parts("OTMSDBZUPAUEDD-UHFFFAOYSA-N",
                                    "CC",
                                    "InChI=1S/C2H6/c1-2/h1-2H3",
                                    json!({"weight": 30.07, "logP": 1.81})).unwrap();
    let family_id = service.create_family_from_molecules(vec![mol1, mol2], json!({})).unwrap();
    let stats = service.calculate_family_statistics(&family_id).unwrap();
    assert_eq!(stats.molecule_count, 2);
    let weight_stats = stats.property_statistics.get("weight").unwrap();
    assert_eq!(weight_stats.count, 2);
    assert!((weight_stats.mean - 38.07).abs() < 0.01);
  }
  #[test]
  fn test_calculate_diversity_metrics() {
    let repo = InMemoryDomainRepository::new();
    let service = FamilyService::new(repo);
    let mol1 = Molecule::from_parts("LFQSCWFLJHTTHZ-UHFFFAOYSA-N",
                                    "CCO",
                                    "InChI=1S/C2H6O/c1-2-3/h3H,2H2,1H3",
                                    json!({})).unwrap();
    let mol2 = Molecule::from_parts("OTMSDBZUPAUEDD-UHFFFAOYSA-N", "CC", "InChI=1S/C2H6/c1-2/h1-2H3", json!({})).unwrap();
    let family_id = service.create_family_from_molecules(vec![mol1, mol2], json!({})).unwrap();
    let diversity = service.calculate_family_diversity(&family_id).unwrap();
    assert_eq!(diversity.total_molecules, 2);
    assert_eq!(diversity.unique_smiles_count, 2);
    assert_eq!(diversity.smiles_diversity_ratio, 1.0);
  }
}
