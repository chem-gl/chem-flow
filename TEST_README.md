````markdown
# Guía de Testing en flow-chem

Este proyecto incluye una estructura mejorada para testing con soporte para mocks, utilidades de testing compartidas y una arquitectura que facilita los tests unitarios e integración.

## Ejecutando Tests con Mocks y Opciones Avanzadas

Este proyecto incluye varias opciones para ejecutar tests en diferentes entornos:

### Tests con mocks (sin RDKit)

Si no tienes RDKit instalado o prefieres tests rápidos sin dependencias externas:

```bash
# Ejecutar todos los tests con mock_rdkit habilitado
cargo test --workspace --features mock_rdkit

# Usar el script de ayuda
./scripts/run_tests_with_mocks.sh
```

### Tests con base de datos

Los tests pueden usar SQLite en memoria (rápido) o PostgreSQL (más completo):

```bash
# Configurar base de datos para tests
./scripts/setup_test_db.sh

# Ejecutar tests con SQLite
export DATABASE_URL="sqlite::memory:"
cargo test --workspace --features sqlite

# Ejecutar tests con PostgreSQL (requiere servidor)
export DATABASE_URL="postgres://usuario:contraseña@localhost/test_db"
cargo test --workspace --features postgres
```

### Ejecutar tests específicos

Para ejecutar categorías específicas de tests:

```bash
# Tests de rehidratación de flujo
cargo test --package flow --test rehydrate_full_flow

# Tests de creación de ramas
cargo test --package flow --test create_branch_from_middle_point

# Tests de inmutabilidad de molécula
cargo test --package chem-domain --test molecule_operations
```

### Tests en contenedores Docker

Para entornos consistentes con todas las dependencias:

```bash
# Usar el script que ejecuta tests en Docker
./scripts/run_tests_in_docker.sh
```

### Generación de Cobertura

Para generar informes de cobertura:

```bash
# Generar reportes en coverage/
./scripts/generate_coverage.sh
```

Los reportes incluyen:
- `coverage/lcov.info`: Para visualización en IDE
- `coverage/cobertura.xml`: Para sistemas CI
- `coverage/sonar-generic-coverage.xml`: Para SonarQube

## Estructura de Testing Mejorada

El proyecto ahora incluye una estructura mejorada para testing:

### Crates y Módulos de Testing

- `crates/chem-utils`: Utilidades compartidas para tests
  - `test_helpers/db_helpers.rs`: Helpers para bases de datos temporales
  - `test_helpers/mock_helpers.rs`: Configuración de mocks para ChemEngine
  - `test_helpers/repository_helpers.rs`: Implementación de TestableRepository

- `crates/flow/tests/`: Tests para el core de flujos
  - `rehydrate_full_flow.rs`: Test de rehidratación completa de flujos
  - `create_branch_from_middle_point.rs`: Test de creación de ramas

- `crates/chem-domain/tests/`: Tests para el dominio químico
  - `molecule_operations.rs`: Tests de inmutabilidad y operaciones de molécula

### Features de Testing

- `mock_rdkit`: Permite ejecutar tests sin necesitar RDKit
- `testing`: Habilita utilidades adicionales para testing

### Traits para Testing

Se han introducido traits para facilitar el testing:

```rust
// TestableRepository - Permite limpiar e inicializar con datos de prueba
pub trait TestableRepository {
    fn clean(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn initialize_with_test_data(&self) -> Result<(), Box<dyn std::error::Error>>;
}

// Implementado para repositorios en memoria
impl TestableRepository for InMemoryDomainRepository {
    // ...
}
```

## Creación de Nuevos Tests

Para crear nuevos tests, sigue estas recomendaciones:

1. **Tests Unitarios**: Colócalos en el módulo `tests` dentro de cada archivo de implementación.
2. **Tests de Integración**: Crea archivos .rs dentro del directorio `tests/` de cada crate.
3. **Usa Mocks**: Para ChemEngine, usa `setup_mock_chem_engine()` de chem-utils.
4. **Bases de Datos Temporales**: Para persistencia, usa `TempSqliteDb::new()`.
5. **Repositorios de Prueba**: Utiliza `create_test_repository()` para obtener un repositorio adecuado para testing.

## Ejemplos

### Test con Mock de ChemEngine

```rust
#[test]
#[cfg(feature = "mock_rdkit")]
fn test_molecule_creation_with_mock() {
    use chem_utils::test_helpers::setup_mock_chem_engine;
    
    let engine = setup_mock_chem_engine();
    let molecule = engine.get_molecule("CCO").unwrap();
    
    assert_eq!(molecule.inchikey, "MOCK-CCO-ABCDEFGHIJ-P");
    assert_eq!(molecule.smiles, "CCO");
}
```

### Test con Base de Datos Temporal

```rust
#[test]
#[cfg(feature = "testing")]
fn test_persistence_with_temp_db() {
    use chem_utils::test_helpers::TempSqliteDb;
    
    let db = TempSqliteDb::new().unwrap();
    std::env::set_var("DATABASE_URL", db.url());
    
    // Usar el repositorio con la base de datos temporal...
}
```

### Test con Repositorio de Prueba

```rust
#[test]
fn test_with_testable_repository() {
    use chem_utils::test_helpers::{create_test_repository, TestableRepository};
    
    let repo = create_test_repository();
    repo.initialize_with_test_data().unwrap();
    
    // Realizar operaciones en el repositorio...
    
    repo.clean().unwrap();
}
```