//! Helpers para mocks en pruebas

#[cfg(feature = "mock_rdkit")]
use chem_providers::test_utils::create_mock_engine;
#[cfg(feature = "mock_rdkit")]
use chem_providers::ChemEngineInterface;

/// Configura un motor químico simulado para pruebas
#[cfg(feature = "mock_rdkit")]
pub fn setup_mock_chem_engine() -> impl ChemEngineInterface {
  create_mock_engine()
}
