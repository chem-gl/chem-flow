// errors.rs
//! Errores del dominio con contexto exhaustivo

use thiserror::Error;

/// Errores del dominio químico (exhaustivos)
#[derive(Debug, Error, Clone)]
pub enum DomainError {
  // === Errores de Validación ===
  /// Entidad no encontrada
  #[error("Entidad {entity_type} con ID '{id}' no encontrada")]
  NotFound { entity_type: String, id: String },

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
  pub fn not_found(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
    Self::NotFound { entity_type: entity_type.into(), id: id.into() }
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
