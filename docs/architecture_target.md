# Arquitectura Objetivo: Hexagonal (Ports and Adapters)

## Visión General

Transformar `flow-chem` a una arquitectura hexagonal donde el dominio es el núcleo aislado, rodeado de ports (interfaces/traits) y adapters (implementaciones) que gestionan toda la infraestructura externa.

## Principios Rectores

### SOLID

- **S (Single Responsibility)**: Cada módulo, struct y función tiene una única razón para cambiar
- **O (Open-Closed)**: Abierto a extensión (nuevos adapters) sin modificar el dominio
- **L (Liskov Substitution)**: Cualquier implementación de un port debe ser intercambiable
- **I (Interface Segregation)**: Traits pequeños y cohesivos, no god-interfaces
- **D (Dependency Inversion)**: El dominio define ports; adapters dependen del dominio, nunca al revés

### Otros Principios

- **DRY**: Eliminar duplicación de lógica de engine
- **KISS**: Mantener simplicidad, evitar sobre-ingeniería
- **YAGNI**: No agregar funcionalidad hasta que sea necesaria
- **Clean Code**: Código auto-documentado con nombres descriptivos

## Diagrama de Arquitectura Objetivo

```
┌────────────────────────────────────────────────────────┐
│                     Entrypoint                         │
│                    (src/main.rs)                       │
│          ┌──────────────────────────────┐              │
│          │   App Container (DI)         │              │
│          │ - Config from env            │              │
│          │ - Wire ports to adapters     │              │
│          └──────────────────────────────┘              │
└──────────────────┬─────────────────────────────────────┘
                   │
    ┌──────────────┼──────────────┐
    │              │              │
    ▼              ▼              ▼
┌───────────┐  ┌──────────┐  ┌──────────┐
│  Diesel   │  │  RDKit   │  │InMemory  │
│  Adapter  │  │  Adapter │  │  Stub    │
└─────┬─────┘  └────┬─────┘  └────┬─────┘
      │             │              │
      │  implements │   implements │
      ▼             ▼              ▼
┌─────────────────────────────────────────┐
│           PORTS (Traits)                │
│  - MoleculeReader                       │
│  - MoleculeWriter                       │
│  - FamilyRepository                     │
│  - PropertyProvider                     │
│  - FlowRepository                       │
│  - SnapshotStore                        │
│  - ArtifactStore                        │
└──────────────┬──────────────────────────┘
               │ defined in
               ▼
┌──────────────────────────────────────────┐
│         DOMAIN CORE                      │
│       (chem-domain)                      │
│  ┌────────────────────────────────┐      │
│  │  Entities                      │      │
│  │  - Molecule                    │      │
│  │  - MoleculeFamily              │      │
│  │  - MolecularProperty           │      │
│  │  - FamilyProperty              │      │
│  └────────────────────────────────┘      │
│  ┌────────────────────────────────┐      │
│  │  Domain Services               │      │
│  │  - MoleculeService             │      │
│  │  - FamilyService               │      │
│  │  - ValidationService           │      │
│  └────────────────────────────────┘      │
│  ┌────────────────────────────────┐      │
│  │  Value Objects                 │      │
│  │  - InChIKey                    │      │
│  │  - SMILES                      │      │
│  │  - PropertyType                │      │
│  └────────────────────────────────┘      │
│  ┌────────────────────────────────┐      │
│  │  Domain Events (future)        │      │
│  │  - MoleculeCreated             │      │
│  │  - FamilyUpdated               │      │
│  └────────────────────────────────┘      │
└──────────────┬───────────────────────────┘
               │ uses ports via
               ▼
┌──────────────────────────────────────────┐
│      WORKFLOW ORCHESTRATION              │
│        (chem-workflow)                   │
│  ┌────────────────────────────────┐      │
│  │  Workflow Engine               │      │
│  │  - Generic orchestration       │      │
│  │  - Step execution              │      │
│  │  - Branching logic             │      │
│  └────────────────────────────────┘      │
│  ┌────────────────────────────────┐      │
│  │  CADMA Flow                    │      │
│  │  - Step1: FamilyRef            │      │
│  │  - Step2: AdmetsaProps         │      │
│  │  - Step3: MoleculeInitial      │      │
│  │  - Step4: AdmetsaInitial       │      │
│  │  - Step5: SubstituteGen        │      │
│  │  - Step6: AdmetsaGenerated     │      │
│  └────────────────────────────────┘      │
│  ┌────────────────────────────────┐      │
│  │  Contexts (specialized)        │      │
│  │  - ReadContext                 │      │
│  │  - WriteContext                │      │
│  │  - PropertyContext             │      │
│  └────────────────────────────────┘      │
└──────────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────┐
│         FLOW ENGINE                      │
│           (flow)                         │
│  - FlowRepository port                   │
│  - Branching/versioning logic            │
│  - Rehydration                           │
└──────────────────────────────────────────┘
```

## Estructura de Crates Objetivo

```
flow-chem/
├── crates/
│   ├── chem-domain/              # Núcleo hexagonal
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── entities/
│   │   │   │   ├── molecule.rs
│   │   │   │   ├── molecule_family.rs
│   │   │   │   ├── molecular_property.rs
│   │   │   │   └── family_property.rs
│   │   │   ├── value_objects/
│   │   │   │   ├── inchikey.rs
│   │   │   │   ├── smiles.rs
│   │   │   │   └── property_type.rs
│   │   │   ├── services/
│   │   │   │   ├── molecule_service.rs
│   │   │   │   ├── family_service.rs
│   │   │   │   └── validation_service.rs
│   │   │   ├── ports/                 # PORTS (traits)
│   │   │   │   ├── molecule_repository.rs
│   │   │   │   ├── family_repository.rs
│   │   │   │   ├── property_provider.rs
│   │   │   │   └── mod.rs
│   │   │   ├── errors.rs
│   │   │   └── events.rs (future)
│   │   └── tests/
│   │       └── unit/                  # Solo tests puros
│   │
│   ├── chem-persistence/         # Adapter de persistencia
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── adapters/
│   │   │   │   ├── diesel_molecule_repository.rs
│   │   │   │   ├── diesel_family_repository.rs
│   │   │   │   └── diesel_flow_repository.rs
│   │   │   ├── config/
│   │   │   │   ├── db_config.rs
│   │   │   │   └── pool.rs
│   │   │   ├── schema.rs
│   │   │   └── migrations.rs
│   │   ├── migrations/
│   │   └── tests/
│   │       └── integration/           # Tests con DB real
│   │
│   ├── chem-providers/           # Adapter de providers externos
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── adapters/
│   │   │   │   ├── rdkit_property_provider.rs
│   │   │   │   └── mock_property_provider.rs
│   │   │   ├── python_bridge/
│   │   │   │   ├── subprocess_runner.rs
│   │   │   │   └── pyo3_wrapper.rs (future)
│   │   │   └── config.rs
│   │   ├── python/
│   │   │   └── rdkit_wrapper.py
│   │   └── tests/
│   │       └── unit/                  # Con mocks
│   │
│   ├── chem-workflow/            # Orquestación (usa ports)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── engine/
│   │   │   │   ├── workflow_engine.rs
│   │   │   │   └── step_executor.rs
│   │   │   ├── context/
│   │   │   │   ├── read_context.rs
│   │   │   │   ├── write_context.rs
│   │   │   │   └── property_context.rs
│   │   │   ├── flows/
│   │   │   │   └── cadma_flow/
│   │   │   │       ├── mod.rs
│   │   │   │       └── steps/
│   │   │   ├── factory/
│   │   │   │   └── workflow_factory.rs
│   │   │   └── errors.rs
│   │   └── tests/
│   │       ├── unit/
│   │       └── e2e/
│   │
│   ├── flow/                     # Engine genérico (agnóstico)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── domain.rs          # DTOs
│   │   │   ├── engine.rs
│   │   │   ├── ports/
│   │   │   │   ├── flow_repository.rs
│   │   │   │   ├── snapshot_store.rs
│   │   │   │   └── artifact_store.rs
│   │   │   ├── stubs.rs           # In-memory para tests
│   │   │   └── errors.rs
│   │   └── tests/
│   │
│   └── chem-utils/               # Helpers compartidos
│       └── src/
│           └── test_helpers/
│
├── src/
│   ├── main.rs                   # Entrypoint + DI
│   ├── app.rs                    # App container
│   └── config.rs                 # Config desde env
│
└── tests/                        # Integration tests globales
    └── e2e/
```

## Definición de Ports

### Ports en `chem-domain/src/ports/`

#### `molecule_repository.rs`

```rust
use crate::entities::Molecule;
use crate::errors::DomainError;

/// Port para lectura de moléculas (Query side)
pub trait MoleculeReader: Send + Sync {
    fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError>;
    fn list_molecules(&self) -> Result<Vec<Molecule>, DomainError>;
    fn find_by_smiles(&self, smiles: &str) -> Result<Vec<Molecule>, DomainError>;
}

/// Port para escritura de moléculas (Command side)
pub trait MoleculeWriter: Send + Sync {
    fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError>;
    fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError>;
}
```

#### `family_repository.rs`

```rust
use crate::entities::{Molecule, MoleculeFamily};
use crate::errors::DomainError;
use uuid::Uuid;

pub trait FamilyRepository: Send + Sync {
    fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError>;
    fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError>;
    fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError>;
    fn delete_family(&self, id: &Uuid) -> Result<(), DomainError>;

    // Operations
    fn add_molecule_to_family(
        &self,
        family_id: &Uuid,
        molecule: Molecule
    ) -> Result<Uuid, DomainError>;
    fn remove_molecule_from_family(
        &self,
        family_id: &Uuid,
        inchikey: &str
    ) -> Result<Uuid, DomainError>;
}
```

#### `property_provider.rs`

```rust
use crate::value_objects::PropertyType;
use crate::entities::MoleculeStructure;
use crate::errors::DomainError;
use std::collections::HashMap;

/// Port para cálculo de propiedades químicas (external provider)
pub trait PropertyProvider: Send + Sync {
    fn calculate_properties(
        &self,
        smiles: &str,
        properties: &[PropertyType]
    ) -> Result<HashMap<PropertyType, f64>, DomainError>;

    fn generate_structure(
        &self,
        smiles: &str
    ) -> Result<MoleculeStructure, DomainError>;

    fn validate_smiles(&self, smiles: &str) -> Result<bool, DomainError>;
}
```

### Ports en `flow/src/ports/`

Ya existen y están bien definidos:

- `FlowRepository`
- `SnapshotStore`
- `ArtifactStore`

## Implementación de Adapters

### Diesel Adapter (`chem-persistence/src/adapters/`)

```rust
// diesel_molecule_repository.rs
pub struct DieselMoleculeRepository {
    pool: Arc<DbPool>,
}

impl MoleculeReader for DieselMoleculeRepository {
    fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError> {
        let mut conn = self.pool.get()?;
        // Diesel query
        Ok(...)
    }
}

impl MoleculeWriter for DieselMoleculeRepository {
    fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError> {
        let mut conn = self.pool.get()?;
        // Transacción Diesel
        Ok(...)
    }
}
```

### RDKit Adapter (`chem-providers/src/adapters/`)

```rust
// rdkit_property_provider.rs
pub struct RDKitPropertyProvider {
    python_path: String,
}

impl PropertyProvider for RDKitPropertyProvider {
    fn calculate_properties(
        &self,
        smiles: &str,
        properties: &[PropertyType]
    ) -> Result<HashMap<PropertyType, f64>, DomainError> {
        // Llamada a subprocess/PyO3
        Ok(...)
    }
}
```

### Mock Adapter (para tests)

```rust
// mock_property_provider.rs
pub struct MockPropertyProvider {
    results: Arc<Mutex<HashMap<String, HashMap<PropertyType, f64>>>>,
}

impl PropertyProvider for MockPropertyProvider {
    fn calculate_properties(
        &self,
        smiles: &str,
        properties: &[PropertyType]
    ) -> Result<HashMap<PropertyType, f64>, DomainError> {
        let results = self.results.lock().unwrap();
        Ok(results.get(smiles).cloned().unwrap_or_default())
    }
}
```

## Workflow Orchestration

### Contextos Especializados

```rust
// context/read_context.rs
pub struct ReadContext<R>
where R: MoleculeReader + FamilyRepository
{
    molecule_repo: Arc<R>,
}

impl<R> ReadContext<R>
where R: MoleculeReader + FamilyRepository
{
    pub fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>> {
        self.molecule_repo.get_molecule(inchikey)
            .map_err(Into::into)
    }

    pub fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>> {
        self.molecule_repo.get_family(id)
            .map_err(Into::into)
    }
}
```

```rust
// context/write_context.rs
pub struct WriteContext<W>
where W: MoleculeWriter + FamilyRepository
{
    molecule_repo: Arc<W>,
    flow_repo: Arc<dyn FlowRepository>,
}

impl<W> WriteContext<W>
where W: MoleculeWriter + FamilyRepository
{
    pub fn save_molecule(&self, molecule: Molecule) -> Result<String> {
        self.molecule_repo.save_molecule(molecule)
            .map_err(Into::into)
    }

    pub fn persist_step_result(
        &self,
        flow_id: Uuid,
        step_name: &str,
        payload: JsonValue
    ) -> Result<PersistResult> {
        // Lógica de persistencia + dedup
        Ok(...)
    }
}
```

```rust
// context/property_context.rs
pub struct PropertyContext<P>
where P: PropertyProvider
{
    property_provider: Arc<P>,
}

impl<P> PropertyContext<P>
where P: PropertyProvider
{
    pub fn calculate_logp(&self, smiles: &str) -> Result<f64> {
        let props = self.property_provider
            .calculate_properties(smiles, &[PropertyType::LogP])?;
        props.get(&PropertyType::LogP)
            .copied()
            .ok_or_else(|| Error::PropertyNotFound("LogP"))
    }
}
```

### Refactorización de Steps

```rust
// flows/cadma_flow/steps/family_reference_step1.rs
pub struct FamilyReferenceStep;

impl WorkflowStep for FamilyReferenceStep {
    type Input = FamilyRefInput;
    type Output = FamilyRefOutput;

    fn execute<R, W, P>(
        &self,
        read_ctx: &ReadContext<R>,
        write_ctx: &WriteContext<W>,
        _prop_ctx: &PropertyContext<P>,
        input: Self::Input
    ) -> Result<Self::Output>
    where
        R: MoleculeReader + FamilyRepository,
        W: MoleculeWriter + FamilyRepository,
        P: PropertyProvider,
    {
        // 1. Leer familia
        let family = read_ctx.get_family(&input.family_id)?
            .ok_or(Error::FamilyNotFound)?;

        // 2. Validar (lógica de dominio)
        if family.is_empty() {
            return Err(Error::EmptyFamily);
        }

        // 3. Persistir resultado
        let output = FamilyRefOutput {
            family_id: input.family_id,
            molecule_count: family.len()
        };

        write_ctx.persist_step_result(
            input.flow_id,
            "FAMILY_REF",
            serde_json::to_value(&output)?
        )?;

        Ok(output)
    }
}
```

## Inyección de Dependencias en `main.rs`

```rust
// src/config.rs
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub rdkit_python_path: String,
    pub snapshot_dir: String,
    pub artifact_dir: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            rdkit_python_path: env::var("RDKIT_PYTHON_PATH")
                .unwrap_or_else(|_| "python3".into()),
            snapshot_dir: env::var("SNAPSHOT_DIR")
                .unwrap_or_else(|_| "./snapshots".into()),
            artifact_dir: env::var("ARTIFACT_DIR")
                .unwrap_or_else(|_| "./artifacts".into()),
        })
    }
}
```

```rust
// src/app.rs
use chem_domain::ports::{MoleculeReader, MoleculeWriter, FamilyRepository, PropertyProvider};
use flow::ports::FlowRepository;

pub struct App {
    // Ports, no implementaciones concretas
    pub molecule_reader: Arc<dyn MoleculeReader>,
    pub molecule_writer: Arc<dyn MoleculeWriter>,
    pub family_repo: Arc<dyn FamilyRepository>,
    pub property_provider: Arc<dyn PropertyProvider>,
    pub flow_repo: Arc<dyn FlowRepository>,
}

impl App {
    pub fn new(config: AppConfig) -> Result<Self> {
        // Crear adapters
        let diesel_repo = Arc::new(
            chem_persistence::adapters::DieselMoleculeRepository::new(&config.database_url)?
        );

        let rdkit_provider = Arc::new(
            chem_providers::adapters::RDKitPropertyProvider::new(config.rdkit_python_path)
        );

        let flow_repo = Arc::new(
            chem_persistence::adapters::DieselFlowRepository::new(
                &config.database_url,
                &config.snapshot_dir,
                &config.artifact_dir
            )?
        );

        Ok(Self {
            molecule_reader: diesel_repo.clone(),
            molecule_writer: diesel_repo.clone(),
            family_repo: diesel_repo,
            property_provider: rdkit_provider,
            flow_repo,
        })
    }

    pub fn create_cadma_workflow(&self) -> CadmaFlow {
        CadmaFlow::new(
            self.molecule_reader.clone(),
            self.molecule_writer.clone(),
            self.family_repo.clone(),
            self.property_provider.clone(),
            self.flow_repo.clone(),
        )
    }
}
```

```rust
// src/main.rs
mod app;
mod config;

use app::App;
use config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Cargar configuración
    let config = AppConfig::from_env()?;

    // 2. Crear App (DI container)
    let app = App::new(config)?;

    // 3. Crear workflow
    let cadma = app.create_cadma_workflow();

    // 4. Ejecutar
    let flow_id = cadma.start().await?;
    cadma.run(flow_id).await?;

    Ok(())
}
```

## Testing Strategy

### Tests Unitarios (sin infraestructura)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chem_utils::test_helpers::*;

    #[test]
    fn family_reference_step_validates_non_empty_family() {
        // Given
        let read_ctx = create_mock_read_context();
        let write_ctx = create_mock_write_context();
        let prop_ctx = create_mock_property_context();

        let family = MoleculeFamily::new(vec![], json!({}));
        read_ctx.set_family(family);

        let step = FamilyReferenceStep;
        let input = FamilyRefInput { family_id: Uuid::new_v4(), flow_id: Uuid::new_v4() };

        // When
        let result = step.execute(&read_ctx, &write_ctx, &prop_ctx, input);

        // Then
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::EmptyFamily));
    }
}
```

### Tests de Integración (con adapters reales)

```rust
#[test]
fn diesel_molecule_repository_saves_and_retrieves() {
    let db = create_test_db();
    let repo = DieselMoleculeRepository::new(&db.url()).unwrap();

    let molecule = Molecule::from_parts(...);
    let key = repo.save_molecule(molecule.clone()).unwrap();

    let retrieved = repo.get_molecule(&key).unwrap();
    assert_eq!(retrieved, Some(molecule));
}
```

### Tests E2E (workflow completo)

```rust
#[tokio::test]
async fn cadma_flow_e2e_with_real_adapters() {
    let config = AppConfig::from_test_env();
    let app = App::new(config).unwrap();

    let cadma = app.create_cadma_workflow();
    let flow_id = cadma.start().await.unwrap();

    cadma.run(flow_id).await.unwrap();

    // Validaciones
    assert!(app.flow_repo.branch_exists(&flow_id).unwrap());
}
```

## Ventajas de la Arquitectura Hexagonal

1. **Testabilidad**: Tests unitarios sin DB/RDKit usando mocks
2. **Mantenibilidad**: Cambiar Diesel por SQLx solo modifica adapters
3. **Escalabilidad**: Añadir nuevo provider (e.g., ChemAxon) solo requiere nuevo adapter
4. **Claridad**: Separación clara de responsabilidades
5. **Reutilización**: Dominio portable a otros proyectos
6. **Evolución**: Fácil migrar a async, GraphQL, gRPC, etc.

## Roadmap de Migración

Ver `REFACTOR_PLAN.md` para plan detallado de 7 fases.

## Referencias

- Clean Architecture: Robert C. Martin
- Hexagonal Architecture: Alistair Cockburn
- Domain-Driven Design: Eric Evans
- Rust API Guidelines: https://rust-lang.github.io/api-guidelines/
