use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Tipos auxiliares reutilizables para ADMETSA
pub type PropertyValues = HashMap<String, f64>;
pub type ManualValues = HashMap<String, PropertyValues>;
pub type MethodPropertyMap = HashMap<ADMETSAProperty, ADMETSAMethod>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ADMETSAMethod {
  Manual,
  Random1,
  Random2,
  Random3,
  Random4,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ADMETSAProperty {
  LogP,
  PSA,
  AtX,
  HBA,
  HBD,
  RB,
  MR,
  LD50,
  Mutagenicity,
  DevelopmentalToxicity,
  SyntheticAccessibility,
}

pub const REQUIRED_PROPERTIES: [ADMETSAProperty; 11] = [ADMETSAProperty::LogP,
                                                        ADMETSAProperty::PSA,
                                                        ADMETSAProperty::AtX,
                                                        ADMETSAProperty::HBA,
                                                        ADMETSAProperty::HBD,
                                                        ADMETSAProperty::RB,
                                                        ADMETSAProperty::MR,
                                                        ADMETSAProperty::LD50,
                                                        ADMETSAProperty::Mutagenicity,
                                                        ADMETSAProperty::DevelopmentalToxicity,
                                                        ADMETSAProperty::SyntheticAccessibility];

pub const ALL_METHODS: [ADMETSAMethod; 5] =
  [ADMETSAMethod::Manual, ADMETSAMethod::Random1, ADMETSAMethod::Random2, ADMETSAMethod::Random3, ADMETSAMethod::Random4];

impl ADMETSAMethod {
  /// Verifica si este método puede generar la propiedad especificada
  pub const fn can_generate(self, prop: ADMETSAProperty) -> bool {
    use ADMETSAProperty::*;
    matches!((self, prop),
             (Self::Manual, _)
             | (Self::Random1, LogP | PSA | AtX | HBA | HBD | RB | MR)
             | (Self::Random2, LD50 | Mutagenicity | DevelopmentalToxicity | SyntheticAccessibility)
             | (Self::Random3, HBD | RB | MR | LD50 | Mutagenicity)
             | (Self::Random4, _))
  }

  /// Calcula un valor mock para la propiedad especificada
  pub const fn calculate_mock_value(self, prop: ADMETSAProperty) -> f64 {
    match (self, prop) {
      (Self::Random1, ADMETSAProperty::LogP) => 2.5,
      (Self::Random1, ADMETSAProperty::PSA) => 45.0,
      (Self::Random1, ADMETSAProperty::AtX) => 24.0,
      (Self::Random1, ADMETSAProperty::HBA) => 3.0,
      (Self::Random1, ADMETSAProperty::HBD) => 1.0,
      (Self::Random1, ADMETSAProperty::RB) => 5.0,
      (Self::Random1, ADMETSAProperty::MR) => 60.0,

      (Self::Random2, ADMETSAProperty::LD50) => 350.0,
      (Self::Random2, ADMETSAProperty::Mutagenicity) => 0.0,
      (Self::Random2, ADMETSAProperty::DevelopmentalToxicity) => 0.0,
      (Self::Random2, ADMETSAProperty::SyntheticAccessibility) => 3.2,

      (Self::Random3, ADMETSAProperty::HBD) => 2.0,
      (Self::Random3, ADMETSAProperty::RB) => 3.0,
      (Self::Random3, ADMETSAProperty::MR) => 72.0,
      (Self::Random3, ADMETSAProperty::LD50) => 250.0,
      (Self::Random3, ADMETSAProperty::Mutagenicity) => 1.0,

      (Self::Random4, ADMETSAProperty::LogP) => 3.1,
      (Self::Random4, ADMETSAProperty::PSA) => 50.0,
      (Self::Random4, ADMETSAProperty::AtX) => 25.0,
      (Self::Random4, ADMETSAProperty::HBA) => 4.0,
      (Self::Random4, ADMETSAProperty::HBD) => 1.5,
      (Self::Random4, ADMETSAProperty::RB) => 4.0,
      (Self::Random4, ADMETSAProperty::MR) => 65.0,
      (Self::Random4, ADMETSAProperty::LD50) => 300.0,
      (Self::Random4, ADMETSAProperty::Mutagenicity) => 0.5,
      (Self::Random4, ADMETSAProperty::DevelopmentalToxicity) => 0.2,
      (Self::Random4, ADMETSAProperty::SyntheticAccessibility) => 2.8,

      _ => 0.0,
    }
  }
}

impl ADMETSAProperty {
  /// Convierte la propiedad a string para usar como clave
  pub fn as_key(&self) -> String {
    format!("{:?}", self)
  }
}
