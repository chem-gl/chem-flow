//! Modelos de DTOs para la API RESTful
//!
//! Define las estructuras de request/response con validación y schemas OpenAPI

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

// ============================================================================
// DTOs de Request
// ============================================================================

/// Request para iniciar una nueva ejecución de CADMA
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StartCadmaRequest {
  /// Nombre opcional del flujo
  pub name: Option<String>,

  /// Metadata adicional para el flujo
  #[serde(default)]
  pub metadata: serde_json::Value,
}

/// Request para ejecutar un paso específico del workflow
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecuteStepRequest {
  /// Número del paso a ejecutar (0-5)
  pub step_index: usize,

  /// Payload específico del paso (JSON dinámico)
  pub payload: serde_json::Value,
}

// ============================================================================
// DTOs de Response
// ============================================================================

/// Respuesta al iniciar una ejecución
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StartCadmaResponse {
  /// ID único de la ejecución
  pub execution_id: Uuid,

  /// Estado inicial
  pub status: String,

  /// Paso actual (siempre 0 al inicio)
  pub current_step: u8,

  /// Timestamp de creación
  pub created_at: String,
}

/// Estado de una ejecución en curso
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CadmaExecutionStatus {
  /// ID de la ejecución
  pub execution_id: Uuid,

  /// Estado actual (running, completed, failed)
  pub status: String,

  /// Paso actual (0-5)
  pub current_step: u8,

  /// Nombre del paso actual
  pub current_step_name: Option<String>,

  /// Pasos completados
  pub steps_completed: Vec<StepInfo>,

  /// Metadata adicional
  pub metadata: serde_json::Value,

  /// Timestamp de última actualización
  pub updated_at: String,
}

/// Información de un paso completado
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StepInfo {
  /// Índice del paso
  pub index: usize,

  /// Nombre del paso
  pub name: String,

  /// Payload de salida del paso
  pub output: serde_json::Value,

  /// Timestamp de ejecución
  pub executed_at: String,
}

/// Respuesta al ejecutar un paso
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecuteStepResponse {
  /// ID de la ejecución
  pub execution_id: Uuid,

  /// Paso que se ejecutó
  pub step_index: usize,

  /// Nombre del paso
  pub step_name: String,

  /// Resultado del paso
  pub result: serde_json::Value,

  /// Nuevo estado del flujo
  pub status: String,

  /// Nuevo paso actual
  pub current_step: u8,
}

/// Respuesta al cancelar una ejecución
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelExecutionResponse {
  /// ID de la ejecución cancelada
  pub execution_id: Uuid,

  /// Mensaje de confirmación
  pub message: String,

  /// Timestamp de cancelación
  pub cancelled_at: String,
}

// ============================================================================
// DTOs para CADMA Steps - Inputs específicos
// ============================================================================

/// Input para Step1: Referencia de Familia
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Step1InputDto {
  /// IDs de familias existentes a usar
  pub families: Option<Vec<Uuid>>,

  /// SMILES para crear nueva familia
  pub smiles: Option<Vec<String>>,

  /// Nombre de nueva familia
  pub new_family_name: Option<String>,

  /// Descripción de nueva familia
  pub new_family_description: Option<String>,
}

/// Input para Step2: Cálculo de propiedades ADMETSA
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Step2InputDto {
  /// Métodos preferidos para cálculo
  pub preferred_methods: Vec<String>, // "Manual", "Random1", etc.

  /// Valores manuales por SMILES
  pub manual_values: Option<HashMap<String, HashMap<String, f64>>>,
}

/// Input para Step3: Generación de molécula inicial
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Step3InputDto {
  /// Método de generación: "Manual" o "Random"
  pub method: String,

  /// SMILES manual (si method = "Manual")
  pub smiles: Option<String>,

  /// Candidatos para random (si method = "Random")
  pub candidates: Option<Vec<String>>,
}

/// Input para Step4: ADMETSA para molécula inicial
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Step4InputDto {
  /// Override de métodos (opcional)
  pub override_methods: Option<Vec<String>>,

  /// Valores manuales (opcional)
  pub manual_values: Option<HashMap<String, HashMap<String, f64>>>,
}

/// Input para Step5: Generación de sustituyentes
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Step5InputDto {
  /// ID de familia de sustituyentes
  pub substitute_family_id: Option<Uuid>,

  /// SMILES de sustituyentes (alternativa)
  pub substitute_smiles: Option<Vec<String>>,

  /// Máximo número de sustituciones
  pub r_substitutes: usize,

  /// Máximo orden de enlace
  pub num_bounds: usize,

  /// Permitir repetición
  pub repeat: bool,

  /// Guardar moléculas generadas
  pub save_generated: bool,
}

/// Input para Step6: ADMETSA para moléculas generadas
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Step6InputDto {
  /// Override de métodos (opcional)
  pub override_methods: Option<Vec<String>>,

  /// Valores manuales por SMILES (opcional)
  pub manual_values: Option<HashMap<String, HashMap<String, f64>>>,
}

// ============================================================================
// Respuestas genéricas
// ============================================================================

/// Respuesta de éxito genérica
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SuccessResponse {
  pub message: String,
}

/// Respuesta de lista de ejecuciones
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListExecutionsResponse {
  pub executions: Vec<ExecutionSummary>,
  pub total: usize,
}

/// Resumen de ejecución para listados
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionSummary {
  pub execution_id: Uuid,
  pub name: Option<String>,
  pub status: String,
  pub current_step: u8,
  pub created_at: String,
  pub updated_at: String,
}

// ============================================================================
// DTOs de Autenticación y Usuarios
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterUserRequest {
  pub name: String,
  pub email: String,
  pub university: Option<String>,
  pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
  pub email: String,
  pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
  pub token: String,
  pub token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
  pub id: Uuid,
  pub name: String,
  pub email: String,
  pub university: Option<String>,
  pub created_at: String,
  pub updated_at: String,
}

// ============================================================================
// DTOs de Equipos
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTeamRequest {
  pub name: String,
  pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamResponse {
  pub id: Uuid,
  pub name: String,
  pub description: Option<String>,
  pub created_at: String,
  pub updated_at: String,
  #[schema(inline)]
  pub members: Vec<UserResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TeamMemberRequest {
  pub user_id: Uuid,
}

// ============================================================================
// DTOs de Moléculas
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMoleculeRequest {
  pub smiles: String,
  #[serde(default)]
  pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MoleculeResponse {
  pub id: Uuid,
  pub inchikey: String,
  pub smiles: String,
  pub inchi: String,
  pub molecular_formula: Option<String>,
  #[serde(default)]
  pub metadata: serde_json::Value,
  pub created_at: String,
  pub updated_at: String,
}

// ============================================================================
// DTOs de Familias
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFamilyRequest {
  pub name: Option<String>,
  pub description: Option<String>,
  /// InChIKeys de moléculas existentes a incluir
  #[serde(default)]
  pub molecule_inchikeys: Vec<String>,
  /// Provenance/metadata de la familia
  #[serde(default)]
  pub provenance: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FamilyResponse {
  pub id: Uuid,
  pub name: Option<String>,
  pub description: Option<String>,
  #[serde(default)]
  pub provenance: serde_json::Value,
  #[serde(default)]
  pub molecule_inchikeys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AddMoleculeToFamilyRequest {
  /// InChIKey de molécula ya existente
  pub molecule_inchikey: String,
}

// ============================================================================
// DTOs de Propiedades
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateFamilyPropertyRequest {
  pub family_id: Uuid,
  pub property_type: String,
  pub value: serde_json::Value,
  pub quality: Option<String>,
  #[serde(default)]
  pub preferred: bool,
  #[serde(default)]
  pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateMolecularPropertyRequest {
  pub molecule_inchikey: String,
  pub property_type: String,
  pub value: serde_json::Value,
  pub quality: Option<String>,
  #[serde(default)]
  pub preferred: bool,
  #[serde(default)]
  pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MolecularPropertyResponse {
  pub id: Uuid,
  pub molecule_inchikey: String,
  pub property_type: String,
  pub value: serde_json::Value,
  pub quality: Option<String>,
  pub preferred: bool,
  pub value_hash: String,
  #[serde(default)]
  pub metadata: serde_json::Value,
}

// ============================================================================
// DTOs de Control de Acceso
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GrantAccessRequest {
  pub accessor_id: Uuid,
  /// "user" o "team"
  pub accessor_type: String,
}
