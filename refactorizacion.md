# Plan de Refactorización flow-chem
## 🎯 Objetivo
Refactorizar el proyecto flow-chem siguiendo principios SOLID, Clean Architecture y mejores prácticas de Rust, sin romper la funcionalidad existente.
---
## 📊 Análisis de Estado Actual
### Problemas Identificados
#### 1. Violaciones SOLID
- **SRP**: Algunos módulos mezclan lógica de dominio con persistencia
- **DIP**: Dependencias directas en lugar de inversión de control en algunos casos
- **ISP**: Algunos traits son demasiado grandes (acoplamiento innecesario)
#### 2. Deuda Técnica
- Código legacy (`domain_stubs`, compatibilidad hacia atrás)
- Duplicación entre `DomainRepository` viejo y Ports nuevos
- Tests que dependen de implementaciones concretas
- Feature flags inconsistentes
#### 3. Arquitectura
- Mezcla de event sourcing y CRUD en algunos lugares
- Falta de validación centralizada
- Error handling inconsistente
- Falta de documentación en algunos módulos
#### 4. Calidad de Código
- Algunos métodos demasiado largos
- Lógica compleja sin descomponer
- Magic numbers y strings
- Falta de tests unitarios en algunos casos
---
## 🗺️ Roadmap de Refactorización
### Fase 0: Preparación
**Objetivo**: Establecer base segura para refactorizar
#### Tareas:
1. **Crear rama de refactorización**
   ```bash
   git checkout -b refactor/clean-architecture
   ```
2. **Ejecutar suite completa de tests**
   ```bash
   ./scripts/run_tests_in_docker.sh
   cargo test --workspace --all-features
   ```
3. **Generar baseline de cobertura**
   ```bash
   ./scripts/generate_coverage.sh
   # Guardar reporte en artifacts/coverage_baseline/
   ```
4. **Documentar funcionalidad crítica**
   - Identificar flujos críticos que NO deben romperse
   - Crear tests end-to-end si faltan
5. **Setup de CI/CD robusto**
   - Asegurar que tests corren en cada push
   - Configurar SonarQube para detectar regresiones
---
### Fase 1: Limpieza de Domain Layer
**Objetivo**: Consolidar el dominio puro sin dependencias externas
#### 1.1 Eliminar código legacy
**Archivos afectados:**
- `/crates/chem-domain/src/domain_stubs.rs`
**Acciones:**
1. Identificar todos los usos de `DomainStubs` e `InMemoryDomainRepository`
   ```bash
   rg "DomainStubs|InMemoryDomainRepository" --type rust
   ```
2. Migrar tests a usar Ports directamente
   ```rust
   // Antes
   let repo = InMemoryDomainRepository::new();
   // Después
   let repo: Arc<dyn AllDomainPorts> = Arc::new(InMemoryAdapter::new());
   ```
3. Deprecar y marcar para eliminación
   ```rust
   #[deprecated(since = "0.2.0", note = "Use AllDomainPorts trait instead")]
   pub use domain_stubs::InMemoryDomainRepository;
   ```
4. Eliminar después de migración completa
**Tests:**
- Ejecutar `cargo test -p chem-domain` después de cada cambio
- Validar que no hay regresiones
#### 1.2 Refactorizar Molecule
**Archivo:** `/crates/chem-domain/src/molecule.rs`
**Problemas:**
- Método `from_smiles` mezcla validación y construcción
- Falta builder pattern para casos complejos
- Validaciones esparcidas
**Acciones:**
1. **Separar responsabilidades:**
   ```rust
   // Nuevo módulo: molecule/builder.rs
   pub struct MoleculeBuilder {
       smiles: Option<String>,
       inchi: Option<String>,
       inchikey: Option<String>,
       metadata: Value,
   }
   impl MoleculeBuilder {
       pub fn new() -> Self { ... }
       pub fn with_smiles(mut self, smiles: String) -> Self { ... }
       pub fn with_inchi(mut self, inchi: String) -> Self { ... }
       pub fn build(self, provider: &dyn PropertyProvider) -> Result<Molecule> { ... }
   }
   ```
2. **Centralizar validación:**
   ```rust
   // Nuevo módulo: molecule/validation.rs
   pub struct MoleculeValidator;
   impl MoleculeValidator {
       pub fn validate_smiles(smiles: &str) -> Result<()> { ... }
       pub fn validate_inchi(inchi: &str) -> Result<()> { ... }
       pub fn validate_inchikey(key: &str) -> Result<()> { ... }
   }
   ```
3. **Simplificar Molecule:**
   ```rust
   impl Molecule {
       // Constructor privado, usar MoleculeBuilder
       pub(crate) fn new_unchecked(/* ... */) -> Self { ... }
       // Factory methods simples
       pub fn from_parts(/* ... */) -> Result<Self> {
           MoleculeBuilder::new()
               .with_smiles(smiles)
               // ...
               .build_unchecked() // Sin provider para casos simples
       }
   }
   ```
**Tests:**
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn molecule_builder_should_create_valid_molecule() { ... }
    #[test]
    fn molecule_validator_should_reject_invalid_smiles() { ... }
}
```
#### 1.3 Refactorizar MoleculeFamily
**Archivo:** `/crates/chem-domain/src/molecule_family.rs`
**Problemas:**
- Hash calculation embebido
- Falta validación de invariantes al construir
- Operaciones que mutan sin validar
**Acciones:**
1. **Extraer hash logic:**
   ```rust
   // Nuevo módulo: molecule_family/hash.rs
   pub struct FamilyHasher;
   impl FamilyHasher {
       pub fn compute(molecules: &[Molecule], provenance: &Value) -> String {
           // Lógica actual del hash
       }
   }
   ```
2. **Validación estricta:**
   ```rust
   impl MoleculeFamily {
       pub fn new(molecules: Vec<Molecule>, provenance: Value) -> Result<Self> {
           if molecules.is_empty() {
               return Err(DomainError::EmptyFamily);
           }
           // Validar duplicados
           let keys: HashSet<_> = molecules.iter().map(|m| m.inchikey()).collect();
           if keys.len() != molecules.len() {
               return Err(DomainError::DuplicateMolecules);
           }
           let family_hash = FamilyHasher::compute(&molecules, &provenance);
           Ok(Self { /* ... */ })
       }
       pub fn add_molecule(&mut self, molecule: Molecule) -> Result<()> {
           if self.frozen {
               return Err(DomainError::FamilyFrozen);
           }
           if self.contains_molecule(molecule.inchikey()) {
               return Err(DomainError::DuplicateMolecule);
           }
           self.molecules.push(molecule);
           self.recalculate_hash();
           Ok(())
       }
   }
   ```
**Tests:**
```rust
#[test]
fn family_should_reject_empty_molecules() { ... }
#[test]
fn family_should_reject_duplicates() { ... }
#[test]
fn frozen_family_should_reject_mutations() { ... }
```
#### 1.4 Refactorizar Properties
**Archivos:**
- `/crates/chem-domain/src/molecular_property.rs`
- `/crates/chem-domain/src/family_property.rs`
**Problemas:**
- Duplicación entre MolecularProperty y FamilyProperty
- Value hashing repetido
**Acciones:**
1. **Crear abstracción común:**
   ```rust
   // Nuevo: property/common.rs
   pub trait Property {
       fn property_type(&self) -> &str;
       fn value(&self) -> &Value;
       fn quality(&self) -> Option<&str>;
       fn is_preferred(&self) -> bool;
       fn value_hash(&self) -> &str;
       fn verify_integrity(&self) -> Result<()>;
   }
   pub struct PropertyHasher;
   impl PropertyHasher {
       pub fn compute(property_type: &str, value: &Value) -> String {
           // Lógica compartida
       }
   }
   ```
2. **Simplificar implementaciones:**
   ```rust
   impl Property for MolecularProperty {
       // Implementar trait
   }
   impl Property for FamilyProperty {
       // Implementar trait
   }
   ```
#### 1.5 Consolidar Services
**Archivos:** `/crates/chem-domain/src/services/`
**Problemas:**
- Servicios demasiado acoplados a implementaciones
- Falta de transaccionalidad
- Error handling inconsistente
**Acciones:**
1. **Service trait pattern:**
   ```rust
   // services/molecule_service.rs
   #[async_trait]
   pub trait MoleculeServiceTrait: Send + Sync {
       async fn create_from_smiles(&self, smiles: &str) -> Result<Molecule>;
       async fn create_with_properties(&self, smiles: &str, properties: Vec<PropertyType>) -> Result<(Molecule, Vec<MolecularProperty>)>;
   }
   pub struct MoleculeService<P, W>
   where
       P: PropertyProvider,
       W: MoleculeWriter,
   {
       provider: Arc<P>,
       writer: Arc<W>,
   }
   #[async_trait]
   impl<P, W> MoleculeServiceTrait for MoleculeService<P, W>
   where
       P: PropertyProvider + Send + Sync,
       W: MoleculeWriter + Send + Sync,
   {
       async fn create_from_smiles(&self, smiles: &str) -> Result<Molecule> {
           // Lógica con manejo de errores robusto
           let provider_molecule = self.provider
               .validate_structure(smiles)
               .map_err(|e| DomainError::ValidationFailed(e.to_string()))?;
           let molecule = Molecule::from_parts(/* ... */)?;
           self.writer.save_molecule(molecule.clone())
               .map_err(|e| DomainError::PersistenceFailed(e.to_string()))?;
           Ok(molecule)
       }
   }
   ```
2. **Unit of Work pattern (opcional para transacciones):**
   ```rust
   pub struct UnitOfWork<'a> {
       reader: &'a dyn MoleculeReader,
       writer: &'a dyn MoleculeWriter,
       family_repo: &'a dyn FamilyRepository,
       // ... más repositorios
   }
   impl<'a> UnitOfWork<'a> {
       pub fn new(ports: &'a dyn AllDomainPorts) -> Self { ... }
       pub async fn commit(&self) -> Result<()> {
           // Si se implementan transacciones en el futuro
       }
   }
   ```
---
### Fase 2: Refactorizar Flow Engine
**Objetivo**: Simplificar el motor de flujos y mejorar event sourcing
#### 2.1 Separar concerns en domain.rs
**Archivo:** `/crates/flow/src/domain.rs`
**Acciones:**
1. **Separar en módulos:**
   ```
   flow/src/domain/
   ├── mod.rs
   ├── flow_data.rs      // FlowData struct
   ├── flow_meta.rs      // FlowMeta struct
   ├── work_item.rs      // WorkItem struct
   ├── snapshot.rs       // SnapshotMeta
   └── persist_result.rs // PersistResult enum
   ```
2. **Mejorar FlowData:**
   ```rust
   // domain/flow_data.rs
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FlowData {
       pub flow_id: String,
       pub cursor: i64,
       pub key: String,
       pub payload: Value,
       pub metadata: Value,
       pub command_id: Option<String>,
       pub created_at: DateTime<Utc>,
   }
   impl FlowData {
       pub fn builder() -> FlowDataBuilder { ... }
       pub fn is_step_state(&self) -> bool {
           self.key.starts_with("step_state:")
       }
       pub fn extract_step_name(&self) -> Option<&str> {
           self.key.strip_prefix("step_state:")
       }
   }
   pub struct FlowDataBuilder { /* ... */ }
   ```
#### 2.2 Refactorizar Repository
**Archivo:** `/crates/flow/src/repository.rs`
**Problemas:**
- Trait demasiado grande
- Métodos que hacen demasiado
**Acciones:**
1. **Segregar en traits más pequeños:**
   ```rust
   // repository/flow_writer.rs
   pub trait FlowWriter: Send + Sync {
       fn create_flow(&self, name: Option<String>, status: Option<String>, metadata: Value) -> Result<String>;
       fn persist_data(&self, data: FlowData, expected_version: i64) -> Result<PersistResult>;
       fn update_flow_status(&self, flow_id: &str, status: &str) -> Result<()>;
   }
   // repository/flow_reader.rs
   pub trait FlowReader: Send + Sync {
       fn get_flow(&self, flow_id: &str) -> Result<FlowMeta>;
       fn read_data(&self, flow_id: &str, cursor: Option<i64>) -> Result<Vec<WorkItem>>;
       fn count_steps(&self, flow_id: &str) -> Result<i64>;
   }
   // repository/flow_branching.rs
   pub trait FlowBranching: Send + Sync {
       fn create_branch(&self, parent_id: &str, cursor: i64, branch_name: Option<String>) -> Result<String>;
       fn branch_exists(&self, flow_id: &str) -> Result<bool>;
       fn delete_branch(&self, flow_id: &str) -> Result<()>;
   }
   // repository/snapshot_store.rs
   pub trait SnapshotStore: Send + Sync {
       fn save_snapshot(&self, flow_id: &str, cursor: i64, data: Vec<u8>) -> Result<String>;
       fn load_latest_snapshot(&self, flow_id: &str) -> Result<Option<SnapshotMeta>>;
       fn load_snapshot(&self, snapshot_id: &str) -> Result<Vec<u8>>;
   }
   // Composite para compatibilidad
   pub trait FlowRepository: FlowWriter + FlowReader + FlowBranching + SnapshotStore {}
   ```
#### 2.3 Mejorar InMemoryRepository
**Archivo:** `/crates/flow/src/stubs.rs`
**Acciones:**
1. **Renombrar y reorganizar:**
   ```
   flow/src/adapters/
   └── in_memory/
       ├── mod.rs
       ├── store.rs        // Estructura de datos
       └── repository.rs   // Implementación de traits
   ```
2. **Mejorar thread safety:**
   ```rust
   use dashmap::DashMap;
   use parking_lot::RwLock;
   pub struct InMemoryFlowStore {
       flows: DashMap<String, FlowMeta>,
       flow_data: DashMap<String, Vec<WorkItem>>,
       snapshots: DashMap<String, Vec<SnapshotMeta>>,
       // Usar estructuras más eficientes
   }
   impl InMemoryFlowStore {
       pub fn new() -> Self { ... }
       // Métodos atómicos con locks mínimos
   }
   ```
---
### Fase 3: Refactorizar Persistence Layer
**Objetivo**: Mejorar implementaciones de Diesel y esquema de BD
#### 3.1 Revisar y optimizar Schema
**Archivo:** `/crates/chem-persistence/src/schema.rs`
**Acciones:**
1. **Añadir índices faltantes:**
   ```sql
   -- Nueva migración: 00000000000004_add_indexes
   CREATE INDEX idx_molecules_smiles ON molecules(smiles);
   CREATE INDEX idx_family_members_molecule ON family_members(molecule_inchikey);
   CREATE INDEX idx_flow_data_key ON flow_data(key);
   CREATE INDEX idx_flow_data_cursor ON flow_data(cursor);
   ```
2. **Añadir constraints:**
   ```sql
   ALTER TABLE molecular_properties
   ADD CONSTRAINT check_property_type
   CHECK (property_type IN ('molecular_weight', 'logp', 'tpsa', ...));
   ```
3. **Revisar tipos de datos:**
   - Usar JSONB en PostgreSQL para mejor performance
   - Considerar tipos específicos para hashes
#### 3.2 Refactorizar DomainPersistence
**Archivo:** `/crates/chem-persistence/src/domain_persistence.rs`
**Problemas:**
- Métodos muy largos
- Transacciones no explícitas
- Error handling genérico
**Acciones:**
1. **Separar en módulos:**
   ```
   chem-persistence/src/diesel_adapter/
   ├── mod.rs
   ├── connection.rs      // Pool y gestión de conexiones
   ├── molecule.rs        // MoleculeReader + MoleculeWriter impl
   ├── family.rs          // FamilyRepository impl
   ├── property.rs        // PropertyRepository impl
   └── transaction.rs     // Helpers para transacciones
   ```
2. **Pattern de transacciones explícitas:**
   ```rust
   // transaction.rs
   pub trait Transactional {
       fn with_transaction<F, R>(&self, f: F) -> Result<R>
       where
           F: FnOnce(&mut PgConnection) -> Result<R>;
   }
   // En implementations
   impl MoleculeWriter for DieselMoleculeAdapter {
       fn save_molecule(&self, molecule: Molecule) -> Result<String> {
           self.with_transaction(|conn| {
               // Operaciones atómicas
               diesel::insert_into(molecules::table)
                   .values(&row)
                   .execute(conn)?;
               Ok(molecule.inchikey().to_string())
           })
       }
   }
   ```
3. **Mejorar error handling:**
   ```rust
   #[derive(Debug, thiserror::Error)]
   pub enum PersistenceError {
       #[error("Database connection failed: {0}")]
       ConnectionFailed(String),
       #[error("Entity not found: {entity_type} with id {id}")]
       NotFound { entity_type: String, id: String },
       #[error("Constraint violation: {0}")]
       ConstraintViolation(String),
       #[error("Transaction failed: {0}")]
       TransactionFailed(String),
       #[error(transparent)]
       DieselError(#[from] diesel::result::Error),
   }
   ```
#### 3.3 Refactorizar FlowPersistence
**Archivo:** `/crates/chem-persistence/src/flow_persistence.rs`
**Similares acciones a domain_persistence, enfocado en:**
- Optimistic locking robusto
- Queries eficientes para read_data
- Batch inserts para flow_data
---
### Fase 4: Refactorizar Providers
**Objetivo**: Mejorar integración con RDKit y testing
#### 4.1 Mejorar RDKit Wrapper
**Archivos:**
- `/crates/chem-providers/python/rdkit_wrapper.py`
- `/crates/chem-providers/src/core.rs`
**Acciones:**
1. **Añadir error handling en Python:**
   ```python
   # rdkit_wrapper.py
   import logging
   from dataclasses import dataclass
   from typing import Optional
   @dataclass
   class MoleculeResult:
       success: bool
       data: Optional[dict]
       error: Optional[str]
   def molecule_info_safe(smiles: str) -> MoleculeResult:
       try:
           mol = Chem.MolFromSmiles(smiles)
           if mol is None:
               return MoleculeResult(
                   success=False,
                   data=None,
                   error=f"Invalid SMILES: {smiles}"
               )
           # ... cálculos
           return MoleculeResult(success=True, data={...}, error=None)
       except Exception as e:
           logging.error(f"RDKit error: {e}")
           return MoleculeResult(success=False, data=None, error=str(e))
   ```
2. **Mejorar binding Rust:**
   ```rust
   // core.rs
   pub struct RDKitProvider {
       py_module: PyObject,
   }
   impl RDKitProvider {
       pub fn new() -> Result<Self, ProviderError> {
           Python::with_gil(|py| {
               let module = PyModule::import(py, "rdkit_wrapper")
                   .map_err(|e| ProviderError::InitializationFailed(e.to_string()))?;
               Ok(Self {
                   py_module: module.into()
               })
           })
       }
       fn call_python_safe<T>(&self, func_name: &str, args: impl IntoPy<Py<PyTuple>>) -> Result<T, ProviderError>
       where
           T: for<'a> FromPyObject<'a>,
       {
           Python::with_gil(|py| {
               let result = self.py_module
                   .getattr(py, func_name)
                   .and_then(|f| f.call1(py, args))
                   .map_err(|e| ProviderError::PythonCallFailed {
                       function: func_name.to_string(),
                       error: e.to_string(),
                   })?;
               result.extract(py)
                   .map_err(|e| ProviderError::DeserializationFailed(e.to_string()))
           })
       }
   }
   ```
#### 4.2 Implementar Mock completo
**Objetivo**: Mock realista para testing sin RDKit
**Acciones:**
1. **Crear mock provider:**
   ```rust
   // chem-providers/src/mock.rs
   pub struct MockPropertyProvider {
       molecule_db: HashMap<String, ProviderMolecule>,
   }
   impl MockPropertyProvider {
       pub fn with_preloaded_molecules(molecules: Vec<(&str, ProviderMolecule)>) -> Self {
           Self {
               molecule_db: molecules.into_iter()
                   .map(|(k, v)| (k.to_string(), v))
                   .collect()
           }
       }
       pub fn with_default_molecules() -> Self {
           // Moléculas comunes para testing
           Self::with_preloaded_molecules(vec![
               ("CCO", ProviderMolecule { /* etanol */ }),
               ("CC", ProviderMolecule { /* etano */ }),
               // ...
           ])
       }
   }
   impl PropertyProvider for MockPropertyProvider {
       fn validate_structure(&self, smiles: &str) -> Result<ProviderMolecule> {
           self.molecule_db.get(smiles)
               .cloned()
               .ok_or_else(|| ProviderError::InvalidStructure(smiles.to_string()))
       }
       // ... otras implementaciones deterministas
   }
   ```
---
### Fase 5: Refactorizar Workflows
**Objetivo**: Simplificar motor de workflows y steps
#### 5.1 Refactorizar ChemicalFlowEngine
**Archivo:** `/crates/chem-workflow/src/engine/chemical_flow.rs`
**Problemas:**
- Método execute demasiado largo
- Estado mezclado con lógica
- Falta de extensibilidad
**Acciones:**
1. **Separar state management:**
   ```rust
   // engine/state.rs
   pub struct EngineState {
       context: StepContext,
       completed_steps: Vec<String>,
       current_cursor: i64,
   }
   impl EngineState {
       pub fn new(flow_id: String, ports: Arc<dyn AllDomainPorts>) -> Self { ... }
       pub fn advance(&mut self, step_name: String) {
           self.completed_steps.push(step_name);
           self.current_cursor += 1;
       }
       pub fn can_execute_step(&self, step_name: &str) -> bool {
           // Verificar dependencias
       }
   }
   ```
2. **Extraer step executor:**
   ```rust
   // engine/executor.rs
   pub struct StepExecutor {
       flow_repo: Arc<dyn FlowRepository>,
       state: EngineState,
   }
   impl StepExecutor {
       pub async fn execute_step<S: WorkflowStep>(&mut self, step: &S) -> Result<()> {
           // 1. Prepare context
           let context = step.prepare_context(&self.state.context, &self.flow_repo).await?;
           // 2. Execute
           let (payload, metadata) = step.execute(context).await?;
           // 3. Persist
           let flow_data = step.create_flow_data(
               &self.state.context.flow_id,
               self.state.current_cursor,
               payload,
               metadata,
           );
           self.persist_with_retry(flow_data).await?;
           // 4. Update state
           self.state.advance(step.name().to_string());
           Ok(())
       }
       async fn persist_with_retry(&self, data: FlowData) -> Result<()> {
           const MAX_RETRIES: usize = 3;
           for attempt in 0..MAX_RETRIES {
               match self.flow_repo.persist_data(data.clone(), self.state.current_cursor).await? {
                   PersistResult::Ok => return Ok(()),
                   PersistResult::Conflict => {
                       if attempt == MAX_RETRIES - 1 {
                           return Err(WorkflowError::PersistConflict);
                       }
                       // Retry logic
                   }
               }
           }
           unreachable!()
       }
   }
   ```
3. **Simplificar ChemicalFlowEngine:**
   ```rust
   pub struct ChemicalFlowEngine {
       executor: StepExecutor,
       workflow_registry: HashMap<WorkflowType, Vec<Box<dyn WorkflowStep>>>,
   }
   impl ChemicalFlowEngine {
       pub async fn execute_workflow(&mut self, workflow_type: WorkflowType) -> Result<()> {
           let steps = self.workflow_registry
               .get(&workflow_type)
               .ok_or(WorkflowError::UnknownWorkflow)?;
           for step in steps {
               self.executor.execute_step(step.as_ref()).await?;
           }
           self.save_final_snapshot().await?;
           Ok(())
       }
   }
   ```
#### 5.2 Refactorizar Steps
**Archivos:** `/crates/chem-workflow/src/flows/cadma_flow/steps/`
**Para cada step:**
1. **Separar validación:**
   ```rust
   // step1/validation.rs
   pub struct Step1Validator;
   impl Step1Validator {
       pub fn validate_input(input: &Step1Input) -> Result<()> {
           if input.principal_smiles.is_empty() {
               return Err(WorkflowError::InvalidInput("Empty SMILES".into()));
           }
           // ... más validaciones
           Ok(())
       }
   }
   ```
2. **Separar lógica de negocio:**
   ```rust
   // step1/processor.rs
   pub struct Step1Processor<'a> {
       context: &'a StepContext,
       input: Step1Input,
   }
   impl<'a> Step1Processor<'a> {
       pub async fn process(self) -> Result<Step1Output> {
           // Lógica limpia y testeable
       }
   }
   ```
3. **Simplificar step implementation:**
   ```rust
   // step1/mod.rs
   pub struct Step1;
   #[async_trait]
   impl WorkflowStep for Step1 {
       async fn execute(&self, mut context: StepContext) -> Result<(Value, Value)> {
           // Obtener input
           let input = context.get_input::<Step1Input>()?;
           // Validar
           Step1Validator::validate_input(&input)?;
           // Procesar
           let output = Step1Processor::new(&context, input)
               .process()
               .await?;
           // Serializar
           let payload = serde_json::to_value(&output)?;
           let metadata = self.create_metadata(&output);
           Ok((payload, metadata))
       }
   }
   ```
#### 5.3 Mejorar Context
**Archivo:** `/crates/chem-workflow/src/step/context.rs`
**Acciones:**
1. **Type-safe access:**
   ```rust
   pub struct StepContext {
       flow_id: String,
       ports: Arc<dyn AllDomainPorts>,
       state: HashMap<String, Value>,
       typed_cache: HashMap<TypeId, Box<dyn Any>>,
   }
   impl StepContext {
       pub fn get<T: 'static + DeserializeOwned>(&self, key: &str) -> Result<T> {
           // Type-safe retrieval
       }
       pub fn set<T: 'static + Serialize>(&mut self, key: &str, value: T) -> Result<()> {
           // Type-safe storage
       }
       pub fn get_cached<T: 'static>(&self) -> Option<&T> {
           self.typed_cache.get(&TypeId::of::<T>())
               .and_then(|boxed| boxed.downcast_ref::<T>())
       }
       pub fn cache<T: 'static>(&mut self, value: T) {
           self.typed_cache.insert(TypeId::of::<T>(), Box::new(value));
       }
   }
   ```
---
### Fase 6: Testing & Quality
**Objetivo**: Mejorar cobertura y calidad de tests
#### 6.1 Añadir tests unitarios faltantes
**Para cada módulo refactorizado:**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    // Builder pattern para test fixtures
    struct MoleculeFixture {
        smiles: String,
        inchi: String,
        inchikey: String,
    }
    impl MoleculeFixture {
        fn ethanol() -> Self { /* ... */ }
        fn caffeine() -> Self { /* ... */ }
        fn build(self) -> Molecule { /* ... */ }
    }
    #[test]
    fn test_molecule_creation_success() {
        // Given
        let fixture = MoleculeFixture::ethanol();
        // When
        let molecule = fixture.build();
        // Then
        assert_eq!(molecule.smiles(), "CCO");
        assert!(molecule.verify_integrity().is_ok());
    }
    #[test]
    fn test_molecule_creation_fails_on_invalid_inchikey() {
        // Given
        let mut fixture = MoleculeFixture::ethanol();
        fixture.inchikey = "INVALID".to_string();
        // When
        let result = std::panic::catch_unwind(|| fixture.build());
        // Then
        assert!(result.is_err());
    }
}
```
#### 6.2 Añadir tests de integración
```rust
// crates/chem-domain/tests/integration/molecule_lifecycle.rs
#[tokio::test]
async fn test_full_molecule_lifecycle() {
    // Setup
    let container = setup_test_container().await;
    let service = container.molecule_service();
    // Create
    let molecule = service.create_from_smiles("CCO").await.unwrap();
    // Read
    let retrieved = service.get_molecule(molecule.inchikey()).await.unwrap();
    assert_eq!(retrieved.smiles(), molecule.smiles());
    // Update (via properties)
    let property = service.add_property(
        molecule.inchikey(),
        PropertyType::MolecularWeight,
    ).await.unwrap();
    // Delete
    service.delete_molecule(molecule.inchikey()).await.unwrap();
    assert!(service.get_molecule(molecule.inchikey()).await.is_err());
}
```
#### 6.3 Property-based testing
```rust
// Usar proptest para testing basado en propiedades
use proptest::prelude::*;
proptest! {
    #[test]
    fn molecule_hash_is_deterministic(smiles in "[A-Z][a-z]?[A-Z][a-z]?") {
        let mol1 = create_molecule_from_smiles(&smiles);
        let mol2 = create_molecule_from_smiles(&smiles);
        prop_assert_eq!(mol1.inchikey(), mol2.inchikey());
    }
    #[test]
    fn family_hash_independent_of_order(
        mol_smiles in prop::collection::vec("[A-Z]{1,3}", 2..10)
    ) {
        let molecules1 = create_molecules(&mol_smiles);
        let mut molecules2 = molecules1.clone();
        molecules2.reverse();
        let family1 = MoleculeFamily::new(molecules1, json!({})).unwrap();
        let family2 = MoleculeFamily::new(molecules2, json!({})).unwrap();
        prop_assert_eq!(family1.family_hash(), family2.family_hash());
    }
}
```
---
### Fase 7: Documentación
**Objetivo**: Actualizar toda la documentación
#### 7.1 Actualizar READMEs
**Para cada crate:**
- Ejemplos actualizados
- API documentation con rustdoc
- Architecture Decision Records (ADR)
```markdown
# ADR-001: Separación de Ports en traits granulares
## Estado
Aceptado
## Contexto
El trait `DomainRepository` original violaba ISP al obligar a implementar
métodos no necesarios.
## Decisión
Separar en `MoleculeReader`, `MoleculeWriter`, `FamilyRepository` y
`PropertyRepository`.
## Consecuencias
### Positivas
- Mejor testabilidad
- Implementaciones parciales posibles
- Más flexible para mocks
### Negativas
- Más traits que gestionar
- Requiere migraciones en código existente
```
#### 7.2 Generar documentación API
```bash
# Generar rustdoc
cargo doc --workspace --no-deps --document-private-items
# Configurar CI para publicar en GitHub Pages
```
---
### Fase 8: Optimización y Performance (Opcional)
#### 8.1 Profiling
```bash
# Instalar flamegraph
cargo install flamegraph
# Profile workflow execution
sudo cargo flamegraph --example cadma_example
# Analizar cuellos de botella
```
#### 8.2 Optimizaciones específicas
1. **Batch operations en persistence:**
   ```rust
   impl MoleculeWriter for DieselMoleculeAdapter {
       fn save_molecules_batch(&self, molecules: Vec<Molecule>) -> Result<Vec<String>> {
           self.with_transaction(|conn| {
               diesel::insert_into(molecules::table)
                   .values(&rows)
                   .execute(conn)?;
               Ok(molecules.iter().map(|m| m.inchikey().to_string()).collect())
           })
       }
   }
   ```
2. **Caching en PropertyProvider:**
   ```rust
   pub struct CachedPropertyProvider<P> {
       inner: P,
       cache: Arc<Mutex<LruCache<String, ProviderMolecule>>>,
   }
   ```
3. **Lazy loading de familias:**
   ```rust
   pub struct LazyMoleculeFamily {
       id: Uuid,
       // ... metadata
       molecules: OnceCell<Vec<Molecule>>,
       loader: Box<dyn Fn() -> Result<Vec<Molecule>>>,
   }
   ```
---
## 📋 Checklist de Validación por Fase
### ✅ Antes de cada commit:
- [ ] **Formato**: `cargo fmt --all` ejecutado (verifica que no hay cambios pendientes)
- [ ] **Linting**: `cargo clippy --workspace --all-features -- -D warnings` sin errores
- [ ] **Tests unitarios**: `cargo test -p <crate>` todos pasan
- [ ] **Tests de integración**: `cargo test --workspace` todos pasan
- [ ] **Coverage**: `./scripts/generate_coverage.sh` - verificar que no disminuye más del 2%
- [ ] **Documentación**: Actualizar comentarios y rustdoc en código modificado
- [ ] **Build**: `cargo build --workspace --all-features` compila sin warnings
### 🔍 Revisiones Manuales Obligatorias por Fase
#### Fase 1: Domain Layer - Checklist de Revisión Manual
**1.1 Molecule.rs - Verificaciones**:
- [ ] Abrir `crates/chem-domain/src/molecule.rs` y revisar:
  - [ ] ¿El método `from_parts()` valida que inchikey no esté vacío? (línea ~85)
  - [ ] ¿El método `verify_integrity()` usa el mismo algoritmo de hash que la creación? (línea ~200)
  - [ ] ¿Todos los campos obligatorios tienen validación? (smiles, inchi, inchikey)
  - [ ] ¿El struct tiene `#[derive(Debug, Clone, Serialize, Deserialize)]`? (línea ~10)
- [ ] Ejecutar test específico: `cargo test -p chem-domain molecule::tests --nocapture`
- [ ] **Verificación manual**: Crear molécula de test y verificar que el hash coincide:
  ```bash
  echo "CCO" | sha256sum  # Comparar con lo que genera el código
  ```
**1.2 MoleculeFamily.rs - Verificaciones**:
- [ ] Abrir `crates/chem-domain/src/molecule_family.rs` y revisar:
  - [ ] ¿El método `add_molecule()` verifica duplicados antes de agregar? (línea ~140)
  - [ ] ¿El método recalcula el hash después de add/remove? (línea ~150, 180)
  - [ ] ¿Se respeta el flag `frozen`? Buscar `if self.frozen` en el código
  - [ ] ¿El constructor `new()` rechaza vectores vacíos? (línea ~60)
- [ ] **Test manual en REPL**:
  ```rust
  // En un test o ejemplo:
  let family = MoleculeFamily::new(vec![], json!({}));
  // Debe retornar Err(DomainError::EmptyFamily)
  ```
**1.3 Properties - Verificaciones**:
- [ ] Comparar `molecular_property.rs` y `family_property.rs`:
  - [ ] ¿Comparten la misma lógica de `value_hash`?
  - [ ] ¿Ambos tienen `verify_integrity()`?
  - [ ] ¿El campo `preferred` tiene un default razonable?
- [ ] **Verificación de duplicación**: Buscar código repetido con:
  ```bash
  rg "value_hash.*sha256" crates/chem-domain/src/ -A 5
  # Si aparece más de una vez, considerar extraer a función compartida
  ```
**1.4 Services - Verificaciones**:
- [ ] Abrir `crates/chem-domain/src/services/molecule_service.rs`:
  - [ ] ¿Todos los errores de provider se convierten a DomainError? (buscar `.map_err`)
  - [ ] ¿Los métodos async usan `await` correctamente?
  - [ ] ¿Se guarda la molécula DESPUÉS de crearla, no antes?
- [ ] **Trace manual del flujo**: Agregar prints y ejecutar:
  ```bash
  # Modificar temporalmente create_from_smiles():
  println!("1. Validando SMILES...");
  // ... validación
  println!("2. Creando molécula...");
  // ... creación
  println!("3. Guardando en BD...");
  # Ejecutar y verificar orden correcto
  ```
#### Fase 2: Flow Engine - Checklist de Revisión Manual
**2.1 FlowData - Verificaciones**:
- [ ] Abrir `crates/flow/src/domain.rs`:
  - [ ] ¿El campo `cursor` es siempre secuencial? (0, 1, 2, 3...)
  - [ ] ¿El campo `command_id` es Option<String>? (para idempotencia)
  - [ ] ¿Los timestamps usan UTC? (línea ~40 buscar `Utc`)
- [ ] **Test de inmutabilidad**: Intentar modificar un FlowData después de crearlo (debería fallar en compilación si no tiene `mut`)
**2.2 Repository Trait - Verificaciones**:
- [ ] Abrir `crates/flow/src/repository.rs`:
  - [ ] ¿El método `persist_data` recibe `expected_version`? (línea ~28)
  - [ ] ¿Retorna `PersistResult` (no Result<(), Error>)? (línea ~30)
  - [ ] ¿Todos los métodos son `&self`, no `&mut self`? (inmutabilidad externa)
- [ ] **Revisión de firmas**: Generar doc y revisar:
  ```bash
  cargo doc -p flow --no-deps --open
  # Navegar a trait FlowRepository y revisar todas las firmas
  ```
**2.3 Optimistic Locking - Verificación Crucial**:
- [ ] Leer implementación en `stubs.rs` línea ~100:
  ```rust
  if self.flows[flow_id].version != expected_version {
      return Ok(PersistResult::Conflict);
  }
  self.flows[flow_id].version += 1;
  ```
- [ ] **Test manual de concurrencia**:
  ```bash
  # Ejecutar test específico:
  cargo test -p flow test_concurrent_writes -- --nocapture
  # Debe mostrar al menos un Conflict
  ```
#### Fase 3: Persistence - Checklist de Revisión Manual
**3.1 Schema Diesel - Verificaciones**:
- [ ] Abrir `crates/chem-persistence/src/schema.rs`:
  - [ ] ¿Todas las tablas tienen primary key definida?
  - [ ] ¿Las foreign keys están presentes? (buscar `joinable!`)
  - [ ] ¿Los índices están declarados? (comparar con migraciones)
- [ ] **Verificar en BD real**:
  ```sql
  -- Conectarse a PostgreSQL:
  docker exec -it flow-chem-db-1 psql -U admin -d mydatabase
  
  -- Listar índices:
  \di
  
  -- Ver constraints:
  SELECT conname, contype FROM pg_constraint WHERE conrelid = 'molecules'::regclass;
  ```
**3.2 DomainPersistence - Verificaciones**:
- [ ] Abrir `crates/chem-persistence/src/domain_persistence.rs`:
  - [ ] Buscar `save_molecule` - ¿usa transacción? (buscar `conn.transaction`)
  - [ ] ¿Se manejan constraint violations? (buscar `diesel::result::Error::DatabaseError`)
  - [ ] ¿Los errores son específicos, no genéricos?
- [ ] **Test de rollback**:
  ```rust
  // En un test:
  // 1. Iniciar transacción
  // 2. Guardar molécula inválida (ej: InChIKey duplicado)
  // 3. Verificar que se hace rollback (molécula no existe en BD)
  ```
**3.3 Migraciones - Verificaciones**:
- [ ] Listar todas las migraciones: `ls crates/chem-persistence/migrations/`
- [ ] **Verificar orden**: Números secuenciales sin gaps
- [ ] **Ejecutar en BD limpia**:
  ```bash
  # 1. Borrar BD de test:
  docker-compose down -v
  docker-compose up -d db
  
  # 2. Ejecutar migraciones:
  diesel migration run --database-url="postgres://admin:admin123@localhost:5432/mydatabase"
  
  # 3. Verificar estado:
  diesel migration list
  ```
- [ ] **Rollback test**:
  ```bash
  diesel migration revert
  # Verificar que las tablas desaparecen
  diesel migration run
  # Verificar que vuelven
  ```
#### Fase 4: Providers - Checklist de Revisión Manual
**4.1 RDKit Wrapper - Verificaciones**:
- [ ] Abrir `crates/chem-providers/python/rdkit_wrapper.py`:
  - [ ] ¿Todas las funciones retornan dict o None (nunca excepciones sin catch)?
  - [ ] ¿Se valida el input? (ej: SMILES no vacío)
  - [ ] ¿Los valores float están redondeados? (evitar problemas de precisión)
- [ ] **Test manual en Python**:
  ```bash
  docker exec -it flow-chem-app-dev-1 python3
  >>> import sys
  >>> sys.path.append('/workspace/crates/chem-providers/python')
  >>> from rdkit_wrapper import molecule_info
  >>> result = molecule_info("INVALID_SMILES")
  >>> print(result)  # Debe ser None o dict con error, NO excepción
  ```
**4.2 PyO3 Binding - Verificaciones**:
- [ ] Abrir `crates/chem-providers/src/core.rs`:
  - [ ] ¿Se capturan todas las PyErr? (buscar `.map_err`)
  - [ ] ¿Se liberan los GIL locks correctamente? (buscar `Python::with_gil`)
  - [ ] ¿Los tipos se convierten correctamente? (ej: PyDict → HashMap)
- [ ] **Test de error handling**:
  ```rust
  // En un test:
  let provider = RDKitPropertyProvider::new().unwrap();
  let result = provider.validate_structure("");  // SMILES vacío
  assert!(result.is_err());
  match result.unwrap_err() {
      ProviderError::InvalidStructure(_) => { /* OK */ }
      _ => panic!("Error type incorrecto"),
  }
  ```
#### Fase 5: Workflows - Checklist de Revisión Manual
**5.1 ChemicalFlowEngine - Verificaciones**:
- [ ] Abrir `crates/chem-workflow/src/engine/chemical_flow.rs`:
  - [ ] ¿El método `execute` maneja errores de cada step? (línea ~80-120)
  - [ ] ¿Se guarda snapshot al final? (buscar `save_snapshot`)
  - [ ] ¿Se reintentan los conflictos de persist? (buscar `retry` o loop)
- [ ] **Trace completo**: Agregar logs en cada punto:
  ```rust
  println!("🔷 Iniciando workflow: {:?}", workflow_type);
  // ... en cada step:
  println!("📍 Ejecutando step: {}", step.name());
  // ... al final:
  println!("✅ Workflow completado. Flow ID: {}", flow_id);
  ```
  Ejecutar y verificar que el orden es correcto.
**5.2 Steps CADMA - Verificaciones**:
- [ ] Para cada step (`step1.rs` a `step5.rs`):
  - [ ] ¿Implementa el trait `WorkflowStep`?
  - [ ] ¿El método `name()` retorna un nombre único?
  - [ ] ¿El método `execute()` es idempotente? (puede ejecutarse varias veces con el mismo input)
  - [ ] ¿Se serializa correctamente el output? (buscar `serde_json::to_value`)
- [ ] **Test de idempotencia**:
  ```rust
  // En un test:
  let step = Step1;
  let context = create_test_context();
  let result1 = step.execute(context.clone()).await.unwrap();
  let result2 = step.execute(context.clone()).await.unwrap();
  assert_eq!(result1, result2);  // Mismo input → mismo output
  ```
**5.3 Context - Verificaciones**:
- [ ] Abrir `crates/chem-workflow/src/step/context.rs`:
  - [ ] ¿Los métodos `get`/`set` son type-safe? (usan generics)
  - [ ] ¿Se manejan keys faltantes? (retornan Result, no panic)
  - [ ] ¿El estado es inmutable desde afuera? (campos privados)
- [ ] **Test de type safety**:
  ```rust
  let mut ctx = StepContext::new(/* ... */);
  ctx.set("test_int", 42_i32).unwrap();
  let val: i32 = ctx.get("test_int").unwrap();
  assert_eq!(val, 42);
  
  // Esto debería fallar en compilación:
  // let val: String = ctx.get("test_int").unwrap();
  ```
### 🧪 Verificaciones de Integración End-to-End
**Después de completar cada fase**, ejecutar este test manual completo:
1. **Limpiar estado**:
   ```bash
   docker-compose down -v
   docker-compose up -d
   sleep 5  # Esperar a que PostgreSQL arranque
   ```
2. **Ejecutar migraciones**:
   ```bash
   docker exec -it flow-chem-app-dev-1 diesel migration run
   ```
3. **Ejecutar todos los tests**:
   ```bash
   ./scripts/run_tests_in_docker.sh
   ```
4. **Ejecutar example completo**:
   ```bash
   ./scripts/run_examples.sh
   # Opción 4: cadma_example
   # Completar el flujo CADMA sin errores
   ```
5. **Verificar en BD**:
   ```sql
   docker exec -it flow-chem-db-1 psql -U admin -d mydatabase -c "
   SELECT 
       (SELECT COUNT(*) FROM molecules) as mol_count,
       (SELECT COUNT(*) FROM molecule_families) as family_count,
       (SELECT COUNT(*) FROM flow_data) as flow_count;
   "
   # Debe mostrar números > 0 en todas las columnas
   ```
6. **Generar coverage**:
   ```bash
   ./scripts/generate_coverage.sh
   ```
   Verificar en `artifacts/coverage/index.html` que no bajó.
### 📊 Checklist de Métricas (Antes vs Después)
Llenar esta tabla antes de empezar y después de cada fase:
| Métrica | Baseline | Fase 1 | Fase 2 | Fase 3 | Fase 4 | Fase 5 | Meta |
|---------|----------|--------|--------|--------|--------|--------|------|
| Tests passing | ___/___  | ___/___ | ___/___ | ___/___ | ___/___ | ___/___ | 100% |
| Coverage % | ___%  | ___% | ___% | ___% | ___% | ___% | >80% |
| Clippy warnings | ___  | ___ | ___ | ___ | ___ | ___ | 0 |
| LOC duplicado | ___  | ___ | ___ | ___ | ___ | ___ | <3% |
| Tiempo CADMA (ms) | ___ms  | ___ms | ___ms | ___ms | ___ms | ___ms | <+5% |
| Memoria uso (MB) | ___MB  | ___MB | ___MB | ___MB | ___MB | ___MB | ~igual |
**Cómo obtener métricas**:
```bash
# Tests:
cargo test --workspace 2>&1 | grep "test result"
# Coverage:
./scripts/generate_coverage.sh
# Ver número en artifacts/coverage/index.html
# Clippy:
cargo clippy --workspace 2>&1 | grep "warning:"| wc -l
# Duplicación:
# Instalar tokei: cargo install tokei
tokei crates/ --files
# Tiempo CADMA:
time docker exec flow-chem-app-dev-1 cargo run -p chem-workflow --example cadma_example
# Memoria:
docker stats flow-chem-app-dev-1 --no-stream
```
### 🔒 Criterios de Aprobación de Fase
Cada fase debe cumplir TODOS estos criterios antes de continuar:
- [ ] **Tests**: 100% de tests pasan (0 failed)
- [ ] **Coverage**: No disminuye más del 2%
- [ ] **Clippy**: 0 warnings con `-D warnings`
- [ ] **Compilación**: `cargo build --workspace --all-features` sin warnings
- [ ] **Documentación**: Todos los ítems públicos documentados (cargo doc warnings = 0)
- [ ] **Code Review**: Al menos 1 otra persona revisó el código
- [ ] **Tests manuales**: Todos los del checklist ejecutados y pasados
- [ ] **Performance**: No más de 5% de degradación
- [ ] **Rollback plan**: Documentado cómo revertir cambios si algo falla
### Antes de merge a main:
- [ ] **Todos los tests en CI pasan** (GitHub Actions, Travis, etc.)
- [ ] **Code review completado** por 2+ reviewers
- [ ] **SonarQube sin issues bloqueantes** (críticos o de seguridad)
- [ ] **Performance no degradada** - benchmarks ejecutados
- [ ] **Documentación completa** - README.md y rustdoc actualizados
- [ ] **CHANGELOG.md actualizado** con todos los cambios
- [ ] **Aprobación del tech lead** o arquitecto del equipo
---
## 🚨 Criterios de Rollback
Si alguna fase causa:
- **> 10% degradación en performance**
- **> 5% caída en coverage**
- **Tests fallando en CI por más de 2 horas**
- **Issues críticos en producción**
➡️ Hacer rollback inmediato y analizar causa raíz
---
## 📈 Métricas de Éxito
### Código:
- **Cobertura**: > 80% (actual: verificar baseline)
- **Complejidad ciclomática**: < 10 por función
- **Duplicación**: < 3%
- **Deuda técnica**: reducción del 50%
### Performance:
- **Tiempo de ejecución CADMA workflow**: < 5% overhead
- **Memoria**: no incremento significativo
- **Queries BD**: reducir N+1 queries a 0
### Mantenibilidad:
- **Tiempo de onboarding**: < 2 días (con esta guía)
- **Tiempo de añadir nuevo step**: < 4 horas
- **Bugs en producción**: 0 críticos tras refactor
---
## 🔄 Plan de Migración Gradual
### Estrategia de Feature Flags
```rust
// Cargo.toml
[features]
default = []
new_domain_api = []
new_persistence = []
```
```rust
// Código de transición
#[cfg(feature = "new_domain_api")]
use crate::ports::AllDomainPorts;
#[cfg(not(feature = "new_domain_api"))]
use crate::domain_stubs::DomainRepository;
```
### Fases de activación:
1. **Semana 1-2**: Features desactivadas, código coexiste
2. **Semana 3**: Activar en tests
3. **Semana 4**: Activar en staging
4. **Semana 5**: Activar en producción
5. **Semana 6**: Eliminar código legacy
---
## 📚 Recursos y Referencias
### Libros recomendados:
- "Clean Architecture" - Robert C. Martin
- "Domain-Driven Design" - Eric Evans
- "Rust for Rustaceans" - Jon Gjengset
### Patterns aplicados:
- Repository Pattern
- Unit of Work
- Builder Pattern
- Strategy Pattern
- Adapter Pattern
- Factory Pattern
### Herramientas:
- `cargo-audit`: Seguridad
- `cargo-outdated`: Dependencias
- `cargo-geiger`: Unsafe code detection
- `cargo-deny`: License compliance
---
**Fecha de inicio**: [A definir]
**Fecha estimada de finalización**: 3-4 semanas
**Responsable**: [Tu nombre]
**Revisores**: [Lista de revisores]
---
## 🎯 Próximos Pasos Inmediatos
1. [ ] Leer y aprobar este plan
2. [ ] Configurar entorno de desarrollo
3. [ ] Ejecutar baseline de tests y cobertura
4. [ ] Crear branch de refactorización
5. [ ] Comenzar Fase 0
---
**¡Buena suerte con la refactorización! 🚀**
---
## 🎯 Apéndice: Plan de Verificación Post-Refactorización
Una vez completada toda la refactorización, ejecuta este plan de verificación final de 4 horas:
### Hora 1: Verificación Automatizada Completa
**1. Clone limpio del repositorio**:
```bash
cd /tmp
git clone <tu-repo> flow-chem-fresh
cd flow-chem-fresh
git checkout refactor/clean-architecture
```
**2. Build desde cero**:
```bash
docker-compose build --no-cache
docker-compose up -d
docker exec -it flow-chem-app-dev-1 cargo clean
docker exec -it flow-chem-app-dev-1 cargo build --workspace --all-features --release
```
- [ ] Compilación exitosa sin warnings
- [ ] Tiempo de compilación razonable (< 5 min en release)
**3. Suite de tests completa**:
```bash
docker exec -it flow-chem-app-dev-1 cargo test --workspace --all-features -- --test-threads=1
```
- [ ] Todos los tests pasan
- [ ] No hay tests ignorados sin justificación
- [ ] Tiempo total < 2 minutos
**4. Análisis estático**:
```bash
docker exec -it flow-chem-app-dev-1 cargo clippy --workspace --all-features -- -D warnings
docker exec -it flow-chem-app-dev-1 cargo fmt --all -- --check
docker exec -it flow-chem-app-dev-1 cargo audit
```
- [ ] 0 warnings de clippy
- [ ] Código formateado correctamente
- [ ] 0 vulnerabilidades de seguridad
### Hora 2: Verificación Funcional Manual
**1. Ejecutar cada ejemplo**:
```bash
./scripts/run_examples.sh
```
Probar CADA opción del menú:
- [ ] Opción 1: example-domain - crear molécula, familia, propiedades
- [ ] Opción 2: example-main - menú completo funciona
- [ ] Opción 3: persistence_simple_usage - persistencia funciona
- [ ] Opción 4: cadma_example - workflow CADMA completo
- [ ] Opción 5: all examples - todos corren sin errores
**2. Verificar datos en BD**:
```sql
docker exec -it flow-chem-db-1 psql -U admin -d mydatabase
-- Verificar integridad de datos:
SELECT COUNT(*) FROM molecules;
SELECT COUNT(*) FROM molecule_families;
SELECT COUNT(*) FROM molecular_properties;
SELECT COUNT(*) FROM flow_data;
-- Verificar constraints:
SELECT * FROM molecules WHERE inchikey IS NULL;  -- debe ser 0 filas
SELECT * FROM molecule_families WHERE family_hash IS NULL;  -- debe ser 0 filas
-- Verificar integridad referencial:
SELECT COUNT(*) FROM family_members fm
LEFT JOIN molecules m ON fm.molecule_inchikey = m.inchikey
WHERE m.inchikey IS NULL;  -- debe ser 0
```
- [ ] Todas las queries retornan valores esperados
- [ ] No hay datos huérfanos o corruptos
**3. Test de stress básico**:
```rust
// Crear archivo: /tmp/stress_test.rs
// En crates/chem-domain/tests/stress_test.rs
#[tokio::test]
async fn test_create_1000_molecules() {
    let repo = setup_test_repo().await;
    let service = MoleculeService::new(repo);
    
    for i in 0..1000 {
        let smiles = format!("C{}", "C".repeat(i % 10));
        let result = service.create_from_smiles(&smiles).await;
        assert!(result.is_ok(), "Failed at iteration {}", i);
    }
}
```
```bash
cargo test stress_test --release -- --nocapture
```
- [ ] Completa sin errors
- [ ] Memoria estable (sin memory leaks)
### Hora 3: Verificación de Arquitectura
**1. Verificar separación de concerns**:
```bash
# Domain no debe depender de persistence:
rg "use.*chem_persistence" crates/chem-domain/src/
# Debe retornar 0 resultados
# Domain no debe depender de providers (excepto en ports):
rg "use.*chem_providers" crates/chem-domain/src/ | grep -v "ports"
# Debe retornar 0 resultados
# Verificar que solo domain usa RDKit:
rg "rdkit|pyo3" crates/ --type rust | grep -v chem-providers
# Debe retornar 0 resultados (excepto en tests)
```
**2. Revisar trait implementations**:
```bash
# Listar todas las implementaciones de ports:
rg "impl.*MoleculeReader" crates/ --type rust -A 2
rg "impl.*MoleculeWriter" crates/ --type rust -A 2
rg "impl.*PropertyProvider" crates/ --type rust -A 2
```
- [ ] DieselDomainRepository implementa todos los traits
- [ ] InMemoryRepository implementa todos los traits
- [ ] RDKitProvider implementa PropertyProvider
**3. Verificar documentación**:
```bash
cargo doc --workspace --no-deps --document-private-items
```
Abrir `target/doc/index.html` y revisar:
- [ ] Todos los módulos públicos tienen doc comments
- [ ] Todos los traits tienen ejemplos de uso
- [ ] No hay "TODO" o "FIXME" en la documentación pública
### Hora 4: Verificación de Calidad y Regresiones
**1. Comparar con baseline**:
```bash
# Generar coverage actual:
./scripts/generate_coverage.sh
# Comparar con baseline guardado:
diff artifacts/coverage_baseline/summary.txt artifacts/coverage/summary.txt
```
- [ ] Coverage no bajó más del 2%
- [ ] Líneas críticas siguen cubiertas
**2. Benchmark de performance**:
```bash
# Ejecutar CADMA 10 veces y promediar:
for i in {1..10}; do
  time docker exec flow-chem-app-dev-1 cargo run -p chem-workflow --example cadma_example --release 2>&1 | grep "real"
done | awk '{sum+=$2} END {print "Average:", sum/NR, "seconds"}'
```
- [ ] Tiempo promedio < baseline + 5%
**3. Revisar deuda técnica**:
```bash
# TODOs y FIXMEs:
rg "TODO|FIXME" crates/ --type rust
# Código comentado:
rg "^\\s*//.*" crates/ --type rust -c | awk '{sum+=$NF} END {print "Commented lines:", sum}'
# Complejidad ciclomática (requiere cargo-complexity):
cargo install cargo-complexity
cargo complexity --all
```
- [ ] TODOs/FIXMEs tienen issue asociado o están resueltos
- [ ] No hay bloques grandes de código comentado
- [ ] Ninguna función con complejidad > 15
**4. Análisis de código duplicado**:
```bash
cargo install cargo-clone
cargo clone check --min-duplicate-lines=5
```
- [ ] < 3% de código duplicado
- [ ] Duplicaciones justificadas (ej: tests)
**5. Security audit**:
```bash
cargo audit
cargo install cargo-geiger
cargo geiger --all-features
```
- [ ] 0 vulnerabilidades conocidas
- [ ] Uso de `unsafe` justificado y mínimo
### ✅ Criterios de Aprobación Final
La refactorización se considera EXITOSA si cumple:
**Funcionalidad** (Peso: 40%):
- [x] Todos los tests pasan (100%)
- [x] Todos los examples funcionan
- [x] No hay regresiones funcionales
- [x] Datos en BD tienen integridad
**Calidad** (Peso: 30%):
- [x] 0 warnings de clippy
- [x] Coverage > 80%
- [x] Documentación completa
- [x] < 3% código duplicado
**Arquitectura** (Peso: 20%):
- [x] Separación de concerns respetada
- [x] Principios SOLID aplicados
- [x] Ports & Adapters implementado correctamente
- [x] Dependencias invertidas
**Performance** (Peso: 10%):
- [x] No más de 5% degradación
- [x] Sin memory leaks
- [x] Tiempo de compilación razonable
### 📊 Scorecard Final
Completa esta tabla al finalizar:
| Categoría | Puntaje (0-10) | Peso | Total |
|-----------|----------------|------|-------|
| Tests passing | ___/10 | 40% | ___ |
| Code quality | ___/10 | 30% | ___ |
| Architecture | ___/10 | 20% | ___ |
| Performance | ___/10 | 10% | ___ |
| **TOTAL** | | | **___/10** |
**Interpretación**:
- **9-10**: Excelente - refactorización exitosa
- **7-8**: Buena - algunos ajustes menores necesarios
- **5-6**: Aceptable - requiere revisión y mejoras
- **< 5**: Insuficiente - considerar rollback y replantear
### 🚨 Plan de Rollback
Si el score final < 7 o hay issues críticos:
1. **Revertir rama**:
   ```bash
   git checkout main
   git branch -D refactor/clean-architecture
   ```
2. **Analizar causa raíz**:
   - Revisar checklist de validación por fase
   - Identificar qué se saltó o hizo mal
   - Documentar lecciones aprendidas
3. **Replanificar**:
   - Dividir en piezas más pequeñas
   - Hacer refactor incremental en lugar de big bang
   - Aumentar cobertura de tests antes de refactorizar
### 📝 Documento de Cierre
Al finalizar exitosamente, crear documento: `docs/refactoring_closure.md`
```markdown
# Refactorización flow-chem - Cierre
## Resumen Ejecutivo
- **Fecha inicio**: ___________
- **Fecha fin**: ___________
- **Duración real**: ___ semanas
- **Score final**: ___/10
## Objetivos Logrados
- [x] Objetivo 1: Limpieza de domain layer
- [x] Objetivo 2: Refactor flow engine
- ...
## Métricas Finales
| Métrica | Antes | Después | Mejora |
|---------|-------|---------|--------|
| Tests passing | ___% | ___% | +___% |
| Coverage | ___% | ___% | +___% |
| ...
## Lecciones Aprendidas
1. ...
2. ...
## Próximos Pasos
1. Monitorear performance en producción
2. Documentar nuevos patterns en wiki
3. ...
```
---
**Con estas verificaciones exhaustivas, tendrás garantía de que la refactorización fue exitosa y no introdujo regresiones. ¡Éxito! 🎉**
