//! # Application Layer
//!
//! Esta capa contiene los casos de uso (use cases) que orquestan la lógica
//! de negocio del dominio. Los use cases son el punto de entrada para las
//! operaciones de la aplicación y coordinan las interacciones entre los
//! servicios de dominio y los puertos.
//!
//! ## Principios de Diseño
//! - Use cases independientes y componibles
//! - Validación de entrada en el boundary
//! - Manejo exhaustivo de errores
//! - Sin dependencias de infraestructura
pub mod use_cases;
pub use use_cases::{
  AddMoleculeToFamilyUseCase, CreateFamilyUseCase, CreateMoleculeUseCase, DeleteFamilyUseCase, DeleteMoleculeUseCase,
  GetFamilyPropertiesUseCase, GetFamilyUseCase, GetMolecularPropertiesUseCase, GetMoleculeUseCase, ListFamiliesUseCase,
  ListMoleculesUseCase, RemoveMoleculeFromFamilyUseCase, SaveFamilyPropertyUseCase, SaveMolecularPropertyUseCase,
};
