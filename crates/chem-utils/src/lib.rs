//! Crate chem-utils - Utilidades compartidas para pruebas y herramientas
//! comunes
//!
//! Este crate proporciona funcionalidades para ayudar con las pruebas y
//! componentes reutilizables en todo el proyecto flow-chem.
// Solo exponer helpers de test durante tests o cuando se habilite la feature
// `testing`
#[cfg(any(test, feature = "testing"))]
pub mod test_helpers;
