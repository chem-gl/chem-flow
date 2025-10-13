//! Helpers para repositorios en pruebas
use chem_domain::InMemoryDomainRepository;
/// Trait para repositorios que pueden ser usados en pruebas
pub trait TestableRepository {
  /// Limpia el repositorio, eliminando todos los datos
  fn clean(&self) -> Result<(), Box<dyn std::error::Error>>;
  /// Inicializa el repositorio con datos para pruebas
  fn initialize_with_test_data(&self) -> Result<(), Box<dyn std::error::Error>>;
}
/// Crea un repositorio en memoria para pruebas
pub fn create_test_repository() -> impl TestableRepository {
  InMemoryDomainRepository::new()
}
impl TestableRepository for InMemoryDomainRepository {
  fn clean(&self) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
  }
  fn initialize_with_test_data(&self) -> Result<(), Box<dyn std::error::Error>> {
    // Aquí podríamos agregar moléculas y familias de prueba
    Ok(())
  }
}
