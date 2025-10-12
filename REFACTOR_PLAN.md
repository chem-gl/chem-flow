# Plan de Refactorización: Arquitectura Hexagonal + SOLID

## Estado Actual del Proyecto

### Estructura de Crates

```
flow-chem/
├── crates/
│   ├── chem-domain/       # Núcleo: moléculas, familias, propiedades
│   ├── chem-persistence/  # Adapter: Diesel/SQLite/Postgres
│   ├── chem-providers/    # Adapter: RDKit via Python/PyO3
│   ├── chem-utils/        # Helpers de testing
│   ├── chem-workflow/     # Orquestación: CADMA flow, steps
│   └── flow/              # Engine genérico de flujos
└── src/main.rs            # Entrypoint
```

### Problemas Identificados

1. **Acoplamiento alto**: `chem-workflow` y `flow` acceden directamente a implementaciones de DB
2. **Violaciones SOLID**:
   - `StepContext` tiene múltiples responsabilidades (S)
   - `DomainRepository` trait demasiado amplio (I)
   - Lógica de negocio mezclada con IO (D)
3. **Código spaghetti**: Lógica dispersa en `engine/`, `factory/`, `flows/`
4. **Testing limitado**: Cobertura ~50%, mocks insuficientes

### Arquitectura Objetivo: Hexagonal

```
                    ┌─────────────────────┐
                    │   Ports (Traits)    │
                    │  MoleculeRepository │
                    │  PropertyProvider   │
                    │  FlowRepository     │
                    └──────────┬──────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
   ┌────▼────┐          ┌──────▼──────┐       ┌──────▼──────┐
   │ Domain  │          │   Workflow  │       │    Flow     │
   │  Core   │◄─────────│   Engine    │◄──────│   Engine    │
   │(chem-   │          │  (chem-     │       │   (flow)    │
   │ domain) │          │  workflow)  │       └─────────────┘
   └─────────┘          └─────────────┘
        ▲                      ▲
        │                      │
   ┌────┴────────────┬─────────┴──────┐
   │                 │                │
┌──▼──────┐    ┌─────▼─────┐   ┌─────▼──────┐
│  Diesel │    │  RDKit    │   │ In-Memory  │
│ Adapter │    │  Adapter  │   │  Adapter   │
│(chem-   │    │(chem-     │   │  (stubs)   │
│persist) │    │providers) │   └────────────┘
└─────────┘    └───────────┘
```

## Fases de Refactorización

### Fase 1: Análisis y Preparación (Estado: PENDIENTE)

**Tareas**:

- [ ] Ejecutar `generate_coverage.sh` y documentar cobertura actual
- [ ] Mapear todas las dependencias circulares o acopladas
- [ ] Crear diagrama UML de arquitectura actual vs. objetivo
- [ ] Identificar violaciones SOLID específicas en cada crate
- [ ] Escribir tests faltantes para alcanzar 70% cobertura base

**Entregables**:

- `docs/architecture_current.md`
- `docs/architecture_target.md`
- `docs/solid_violations.md`

### Fase 2: Núcleo de Dominio Puro (Estado: PENDIENTE)

**Objetivo**: `chem-domain` debe ser 100% agnóstico de infraestructura.

**Tareas**:

- [ ] Auditar imports en `chem-domain/src/`: eliminar `chem-persistence`, `chem-providers`
- [ ] Refactorizar `domain_repository.rs`:

  ```rust
  // Port para lectura
  pub trait MoleculeReader: Send + Sync {
      fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>>;
      fn list_molecules(&self) -> Result<Vec<Molecule>>;
  }

  // Port para escritura
  pub trait MoleculeWriter: Send + Sync {
      fn save_molecule(&self, molecule: Molecule) -> Result<String>;
      fn delete_molecule(&self, inchikey: &str) -> Result<()>;
  }

  // Port para familias (separado por ISP)
  pub trait FamilyRepository: Send + Sync {
      fn save_family(&self, family: MoleculeFamily) -> Result<Uuid>;
      fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>>;
      fn delete_family(&self, id: &Uuid) -> Result<()>;
  }
  ```

- [ ] Refactorizar `errors.rs` a enum exhaustivo con contexto:
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum DomainError {
      #[error("Validation failed: {0}")]
      Validation(String),
      #[error("Entity not found: {entity_type} with id {id}")]
      NotFound { entity_type: String, id: String },
      #[error("External provider error: {0}")]
      ExternalProvider(#[from] anyhow::Error),
  }
  ```
- [ ] Hacer `Molecule`, `MoleculeFamily` inmutables (builder pattern si necesario)
- [ ] Tests unitarios puros (sin DB, sin RDKit): usar `domain_stubs.rs`

**Validación SOLID**:

- **S**: `Molecule` solo representa molécula, no tiene lógica de persistencia
- **O**: Nuevas propiedades vía enum `PropertyType`, no modifica entidades
- **L**: Subtipos de `Property` son intercambiables
- **I**: Traits separados por operación (read/write/family)
- **D**: Ninguna dependencia concreta en dominio

### Fase 3: Adapters de Persistencia (Estado: PENDIENTE)

**Objetivo**: `chem-persistence` implementa ports del dominio.

**Tareas**:

- [ ] Renombrar `DieselFlowRepository` → `DieselMoleculeWriter` (o similar)
- [ ] Implementar ports:

  ```rust
  // En chem-persistence/src/adapters/molecule_repository.rs
  pub struct DieselMoleculeRepository {
      pool: Arc<DbPool>,
  }

  impl MoleculeReader for DieselMoleculeRepository {
      fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>> {
          // usa Diesel query
      }
  }

  impl MoleculeWriter for DieselMoleculeRepository {
      fn save_molecule(&self, molecule: Molecule) -> Result<String> {
          // transacción Diesel
      }
  }
  ```

- [ ] Mover `migrations/` a `chem-persistence/migrations/` (ya está, confirmar)
- [ ] Crear factory para construir repos:
  ```rust
  pub fn create_molecule_repository(db_url: &str)
      -> Result<Box<dyn MoleculeReader + MoleculeWriter>> {
      let pool = create_pool(db_url)?;
      Ok(Box::new(DieselMoleculeRepository::new(pool)))
  }
  ```
- [ ] Tests de integración con DB real (usar `test_helpers.rs` para setup temporal)

**Validación**:

- Dominio no cambia si cambiamos de Diesel a SQLx
- Tests pueden usar in-memory mock sin tocar dominio

### Fase 4: Adapters de Providers Externos (Estado: PENDIENTE)

**Objetivo**: `chem-providers` implementa ports para propiedades químicas.

**Tareas**:

- [ ] Definir port en dominio:
  ```rust
  // En chem-domain/src/ports/property_provider.rs
  pub trait PropertyProvider: Send + Sync {
      fn calculate_properties(&self, smiles: &str, props: &[PropertyType])
          -> Result<HashMap<PropertyType, f64>>;
      fn generate_structure(&self, smiles: &str) -> Result<MoleculeStructure>;
  }
  ```
- [ ] Refactorizar `chem-providers/src/core.rs`:

  ```rust
  pub struct RDKitProvider {
      python_path: String,
  }

  impl PropertyProvider for RDKitProvider {
      fn calculate_properties(&self, smiles: &str, props: &[PropertyType])
          -> Result<HashMap<PropertyType, f64>> {
          // llama rdkit_wrapper.py via subprocess o PyO3
      }
  }
  ```

- [ ] Mock provider para tests:
  ```rust
  pub struct MockPropertyProvider {
      results: HashMap<String, HashMap<PropertyType, f64>>,
  }
  impl PropertyProvider for MockPropertyProvider { /* ... */ }
  ```
- [ ] Tests unitarios usando mock (no requiere Python/RDKit)

### Fase 5: Workflows y Engine Hexagonal (Estado: PENDIENTE)

**Objetivo**: `chem-workflow` y `flow` orquestan vía ports, no impls.

**Tareas en `chem-workflow`**:

- [ ] Refactorizar `StepContext`:
  ```rust
  pub struct StepContext<R, P>
  where
      R: MoleculeReader + FamilyRepository,
      P: PropertyProvider,
  {
      flow_id: Uuid,
      molecule_repo: Arc<R>,
      property_provider: Arc<P>,
      flow_repo: Arc<dyn FlowRepository>,
  }
  ```
- [ ] Convertir steps a traits:
  ```rust
  pub trait WorkflowStep: Send + Sync {
      fn name(&self) -> &'static str;
      fn execute<R, P>(&self, ctx: &StepContext<R, P>) -> StepResult
      where
          R: MoleculeReader + FamilyRepository,
          P: PropertyProvider;
  }
  ```
- [ ] Factory para construir workflows inyectando dependencias:
  ```rust
  pub struct WorkflowFactory<R, P> {
      molecule_repo: Arc<R>,
      property_provider: Arc<P>,
  }
  impl<R, P> WorkflowFactory<R, P> {
      pub fn create_cadma_flow(&self, flow_repo: Arc<dyn FlowRepository>)
          -> CadmaFlow<R, P> { /* ... */ }
  }
  ```
- [ ] Eliminar lógica hardcoded de persistencia en `engine/`

**Tareas en `flow`**:

- [ ] `FlowRepository` trait ya bien definido; confirmar que todos los métodos son ports
- [ ] Implementaciones (`InMemoryFlowRepository`, Diesel adapter) usan trait
- [ ] Engine genérico no depende de impls concretas

**Validación**:

- Puedo inyectar mock repos en tests sin cambiar workflows
- Workflows siguen SRP: cada step hace una cosa

### Fase 6: Inyección de Dependencias Central (Estado: PENDIENTE)

**Objetivo**: `src/main.rs` ensambla todo.

**Tareas**:

- [ ] Crear `AppConfig` para leer env:
  ```rust
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
- [ ] Crear `App` container:
  ```rust
  pub struct App {
      molecule_repo: Arc<dyn MoleculeReader + MoleculeWriter>,
      family_repo: Arc<dyn FamilyRepository>,
      property_provider: Arc<dyn PropertyProvider>,
      flow_repo: Arc<dyn FlowRepository>,
  }
  impl App {
      pub fn new(config: AppConfig) -> Result<Self> {
          let molecule_repo = create_molecule_repository(&config.database_url)?;
          let property_provider = Arc::new(RDKitProvider::new(
              config.rdkit_python_path
          ));
          let flow_repo = create_flow_repository(
              &config.database_url,
              &config.snapshot_dir,
              &config.artifact_dir
          )?;
          Ok(Self { molecule_repo, family_repo, property_provider, flow_repo })
      }

      pub fn workflow_factory(&self) -> WorkflowFactory {
          WorkflowFactory::new(
              self.molecule_repo.clone(),
              self.property_provider.clone()
          )
      }
  }
  ```
- [ ] En `main.rs`:
  ```rust
  #[tokio::main]
  async fn main() -> anyhow::Result<()> {
      let config = AppConfig::from_env()?;
      let app = App::new(config)?;

      // Crear workflow inyectando dependencias
      let factory = app.workflow_factory();
      let cadma = factory.create_cadma_flow(app.flow_repo.clone())?;

      // Ejecutar
      cadma.run().await?;
      Ok(())
  }
  ```

**Validación DIP**:

- Ninguna impl concreta en `main.rs`, solo traits
- Cambiar de Diesel a SQLx solo modifica factory, no main

### Fase 7: Tests y Validación Final (Estado: PENDIENTE)

**Tareas**:

- [ ] Expandir `chem-utils/src/test_helpers/`:
  ```rust
  pub fn create_mock_molecule_repo() -> Arc<MockMoleculeRepository> { /* ... */ }
  pub fn create_mock_property_provider() -> Arc<MockPropertyProvider> { /* ... */ }
  ```
- [ ] Escribir tests BDD-style en cada crate:
  ```rust
  #[test]
  fn given_valid_smiles_when_calculate_properties_then_returns_logp() {
      // Given
      let provider = create_mock_property_provider();
      provider.set_result("CCO", PropertyType::LogP, 0.23);

      // When
      let result = provider.calculate_properties("CCO", &[PropertyType::LogP]);

      // Then
      assert!(result.is_ok());
      assert_eq!(result.unwrap().get(&PropertyType::LogP), Some(&0.23));
  }
  ```
- [ ] Aumentar cobertura con `generate_coverage.sh` > 70%
- [ ] Tests E2E con Docker: `docker-compose up && cargo test --workspace`
- [ ] CI/CD: validar en GitHub Actions con `cargo clippy`, `cargo fmt --check`, `cargo test`

## Principios Aplicados por Fase

| Fase | SOLID        | Otros Principios            |
| ---- | ------------ | --------------------------- |
| 1    | -            | TDD base, Clean Code        |
| 2    | Todos        | DRY, KISS, Ownership        |
| 3    | D, I, O      | Error Handling, Modularidad |
| 4    | D, I, L      | YAGNI, Mocks                |
| 5    | S, O, D      | DRY, Inyección Dependencias |
| 6    | D (completo) | Config as Code, DIP         |
| 7    | -            | TDD/BDD, CI/CD              |

## Métricas de Éxito

- [ ] Cobertura de tests > 70%
- [ ] 0 dependencias circulares entre crates
- [ ] Dominio sin imports de infraestructura
- [ ] Todos los tests pasan en Docker
- [ ] Tiempo de compilación < 2min en CI
- [ ] Clippy 0 warnings con `-D warnings`

## Cronograma Estimado

- Fase 1: 1 día
- Fase 2: 2 días
- Fase 3: 2 días
- Fase 4: 1 día
- Fase 5: 3 días
- Fase 6: 1 día
- Fase 7: 2 días

**Total: ~12 días de desarrollo**

## Referencias

- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)
- [SOLID Principles in Rust](https://rust-unofficial.github.io/patterns/)
- [Clean Architecture (Robert C. Martin)](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
