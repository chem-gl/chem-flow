//! Errores del dominio con contexto exhaustivo
use thiserror::Error;
/// Errores del dominio químico (exhaustivos)
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DomainError {
  // === Errores de Validación ===
  /// Entidad no encontrada
  #[error("Entidad {entity} con ID '{id}' no encontrada")]
  NotFound { entity: String, id: String },
  /// Validación de datos fallida
  #[error("Validación fallida para {entity}: {reason}")]
  ValidationError { entity: String, reason: String },
  /// Formato inválido
  #[error("Formato inválido para {field}: {value}. Razón: {reason}")]
  InvalidFormat { field: String, value: String, reason: String },
  /// Constraint de negocio violado
  #[error("Constraint violado: {constraint}. Detalles: {details}")]
  ConstraintViolation { constraint: String, details: String },
  // === Errores de Proveedor Externo ===
  /// Error del proveedor químico (RDKit, ChemAxon, etc.)
  #[error("Error del proveedor químico: {provider}. Detalles: {details}")]
  ProviderError { provider: String, details: String },
  /// Error de cálculo de propiedades
  #[error("Error calculando propiedad {property} para {smiles}: {reason}")]
  PropertyCalculationError { property: String, smiles: String, reason: String },
  // === Errores de Persistencia ===
  /// Error de persistencia (DB, filesystem, etc.)
  #[error("Error de persistencia: {operation}. Detalles: {details}")]
  PersistenceError { operation: String, details: String },
  /// Conflicto de versión (optimistic locking)
  #[error("Conflicto de versión para {entity} con ID {id}. Versión esperada: {expected}, actual: {actual}")]
  VersionConflict { entity: String, id: String, expected: i64, actual: i64 },
  // === Errores de Serialización ===
  /// Error de serialización/deserialización
  #[error("Error de serialización: {context}. Detalles: {details}")]
  SerializationError { context: String, details: String },
  // === Errores de Lógica de Negocio ===
  /// Operación inválida
  #[error("Operación '{operation}' inválida: {reason}")]
  InvalidOperation { operation: String, reason: String },
  /// Estado inválido
  #[error("Estado inválido para {entity}: {current_state}. Razón: {reason}")]
  InvalidState { entity: String, current_state: String, reason: String },
}
// === Conversiones desde errores estándar ===
impl From<serde_json::Error> for DomainError {
  fn from(e: serde_json::Error) -> Self {
    Self::SerializationError { context: "JSON".to_string(), details: e.to_string() }
  }
}
// === Helpers para construcción ergonómica ===
impl DomainError {
  /// Constructor para validación simple
  pub fn validation(entity: impl Into<String>, reason: impl Into<String>) -> Self {
    Self::ValidationError { entity: entity.into(), reason: reason.into() }
  }
  /// Constructor para not found simple
  pub fn not_found(entity: impl Into<String>, id: impl Into<String>) -> Self {
    Self::NotFound { entity: entity.into(), id: id.into() }
  }
  /// Constructor para errores de proveedor
  pub fn provider(provider: impl Into<String>, details: impl Into<String>) -> Self {
    Self::ProviderError { provider: provider.into(), details: details.into() }
  }
  /// Constructor para errores de persistencia
  pub fn persistence(operation: impl Into<String>, details: impl Into<String>) -> Self {
    Self::PersistenceError { operation: operation.into(), details: details.into() }
  }
  /// Constructor para formato inválido
  pub fn invalid_format(field: impl Into<String>, value: impl Into<String>, reason: impl Into<String>) -> Self {
    Self::InvalidFormat { field: field.into(), value: value.into(), reason: reason.into() }
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_validation_error() {
    let err = DomainError::validation("Molecule", "SMILES vacío");
    assert_eq!(err,
               DomainError::ValidationError { entity: "Molecule".to_string(), reason: "SMILES vacío".to_string() });
    assert_eq!(err.to_string(), "Validación fallida para Molecule: SMILES vacío");
  }
  #[test]
  fn test_not_found() {
    let err = DomainError::not_found("MoleculeFamily", "123e4567-e89b-12d3-a456-426614174000");
    assert_eq!(err,
               DomainError::NotFound { entity: "MoleculeFamily".to_string(),
                                       id: "123e4567-e89b-12d3-a456-426614174000".to_string() });
    assert_eq!(err.to_string(),
               "Entidad MoleculeFamily con ID '123e4567-e89b-12d3-a456-426614174000' no encontrada");
  }
  #[test]
  fn test_from_serde_error() {
    let serde_err = serde_json::from_str::<i32>("invalid").unwrap_err();
    let domain_err: DomainError = serde_err.into();
    assert!(matches!(domain_err, DomainError::SerializationError { .. }));
  }
}
