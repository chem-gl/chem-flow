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

### Fase 0: Preparación (1 día)

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

### Fase 1: Limpieza de Domain Layer (3-4 días)

**Objetivo**: Consolidar el dominio puro sin dependencias externas

#### 1.1 Eliminar código legacy (Día 1)

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

#### 1.2 Refactorizar Molecule (Día 1-2)

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

#### 1.3 Refactorizar MoleculeFamily (Día 2)

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

#### 1.4 Refactorizar Properties (Día 3)

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

#### 1.5 Consolidar Services (Día 4)

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

### Fase 2: Refactorizar Flow Engine (2-3 días)

**Objetivo**: Simplificar el motor de flujos y mejorar event sourcing

#### 2.1 Separar concerns en domain.rs (Día 1)

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

#### 2.2 Refactorizar Repository (Día 2)

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

#### 2.3 Mejorar InMemoryRepository (Día 3)

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

### Fase 3: Refactorizar Persistence Layer (3 días)

**Objetivo**: Mejorar implementaciones de Diesel y esquema de BD

#### 3.1 Revisar y optimizar Schema (Día 1)

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

#### 3.2 Refactorizar DomainPersistence (Día 2)

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

#### 3.3 Refactorizar FlowPersistence (Día 3)

**Archivo:** `/crates/chem-persistence/src/flow_persistence.rs`

**Similares acciones a domain_persistence, enfocado en:**

- Optimistic locking robusto
- Queries eficientes para read_data
- Batch inserts para flow_data

---

### Fase 4: Refactorizar Providers (2 días)

**Objetivo**: Mejorar integración con RDKit y testing

#### 4.1 Mejorar RDKit Wrapper (Día 1)

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

#### 4.2 Implementar Mock completo (Día 2)

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

### Fase 5: Refactorizar Workflows (3-4 días)

**Objetivo**: Simplificar motor de workflows y steps

#### 5.1 Refactorizar ChemicalFlowEngine (Día 1-2)

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

#### 5.2 Refactorizar Steps (Día 2-3)

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

#### 5.3 Mejorar Context (Día 4)

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

### Fase 6: Testing & Quality (2-3 días)

**Objetivo**: Mejorar cobertura y calidad de tests

#### 6.1 Añadir tests unitarios faltantes (Día 1)

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

#### 6.2 Añadir tests de integración (Día 2)

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

#### 6.3 Property-based testing (Día 3)

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

### Fase 7: Documentación (1-2 días)

**Objetivo**: Actualizar toda la documentación

#### 7.1 Actualizar READMEs (Día 1)

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

#### 7.2 Generar documentación API (Día 2)

```bash
# Generar rustdoc
cargo doc --workspace --no-deps --document-private-items

# Configurar CI para publicar en GitHub Pages
```

---

### Fase 8: Optimización y Performance (2 días - Opcional)

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

### Antes de cada commit:

- [ ] `cargo fmt` ejecutado
- [ ] `cargo clippy -- -D warnings` sin errores
- [ ] Tests unitarios pasan: `cargo test -p <crate>`
- [ ] Tests de integración pasan: `cargo test --workspace`
- [ ] Coverage no disminuye: `./scripts/generate_coverage.sh`
- [ ] Documentación actualizada

### Antes de merge a main:

- [ ] Todos los tests en CI pasan
- [ ] Code review completado
- [ ] SonarQube sin issues bloqueantes
- [ ] Performance no degradada (benchmarks)
- [ ] Documentación completa
- [ ] CHANGELOG.md actualizado

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
