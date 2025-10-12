# Análisis de Violaciones SOLID en flow-chem

## Resumen Ejecutivo

Este documento detalla las violaciones a los principios SOLID encontradas en el proyecto `flow-chem`, con ejemplos concretos de código y propuestas de refactorización.

**Total de violaciones identificadas**: 15 (5 críticas, 7 altas, 3 medias)

## Principios SOLID

- **S (Single Responsibility Principle)**: Una clase/módulo debe tener una sola razón para cambiar
- **O (Open-Closed Principle)**: Abierto a extensión, cerrado a modificación
- **L (Liskov Substitution Principle)**: Los subtipos deben ser sustituibles por sus tipos base
- **I (Interface Segregation Principle)**: Los clientes no deben depender de interfaces que no usan
- **D (Dependency Inversion Principle)**: Depender de abstracciones, no de concreciones

---

## Violaciones Críticas

### 1. `DomainRepository` - Violación de ISP (Interface Segregation)

**Ubicación**: `crates/chem-domain/src/domain_repository.rs:7-36`

**Problema**: Trait con 13 métodos mezclando 3 concerns diferentes (moléculas, familias, propiedades)

**Código actual**:

```rust
pub trait DomainRepository: Send + Sync {
    // Moléculas (4 métodos)
    fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError>;
    fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError>;
    fn list_molecules(&self) -> Result<Vec<Molecule>, DomainError>;
    fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError>;

    // Familias (6 métodos)
    fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError>;
    fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError>;
    fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError>;
    fn delete_family(&self, id: &Uuid) -> Result<(), DomainError>;
    fn add_molecule_to_family(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError>;
    fn remove_molecule_from_family(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError>;

    // Propiedades (4 métodos)
    fn save_family_property(&self, prop: OwnedFamilyProperty) -> Result<Uuid, DomainError>;
    fn get_family_properties(&self, family_id: &Uuid) -> Result<Vec<OwnedFamilyProperty>, DomainError>;
    fn save_molecular_property(&self, prop: OwnedMolecularProperty) -> Result<Uuid, DomainError>;
    fn get_molecular_properties(&self, inchikey: &str) -> Result<Vec<OwnedMolecularProperty>, DomainError>;
}
```

**Impacto**:

- Clientes que solo necesitan leer moléculas están forzados a implementar/depender de 13 métodos
- Cambios en propiedades obligan a recompilar todo el trait
- Dificulta testing (mocks deben implementar todo)

**Refactorización**:

```rust
// Separar en traits cohesivos (CQRS pattern)

pub trait MoleculeReader: Send + Sync {
    fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>, DomainError>;
    fn list_molecules(&self) -> Result<Vec<Molecule>, DomainError>;
    fn find_by_smiles(&self, smiles: &str) -> Result<Vec<Molecule>, DomainError>;
}

pub trait MoleculeWriter: Send + Sync {
    fn save_molecule(&self, molecule: Molecule) -> Result<String, DomainError>;
    fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError>;
}

pub trait FamilyRepository: Send + Sync {
    fn save_family(&self, family: MoleculeFamily) -> Result<Uuid, DomainError>;
    fn get_family(&self, id: &Uuid) -> Result<Option<MoleculeFamily>, DomainError>;
    fn list_families(&self) -> Result<Vec<MoleculeFamily>, DomainError>;
    fn delete_family(&self, id: &Uuid) -> Result<(), DomainError>;
    fn add_molecule_to_family(&self, family_id: &Uuid, molecule: Molecule) -> Result<Uuid, DomainError>;
    fn remove_molecule_from_family(&self, family_id: &Uuid, inchikey: &str) -> Result<Uuid, DomainError>;
}

pub trait PropertyRepository: Send + Sync {
    fn save_family_property(&self, prop: OwnedFamilyProperty) -> Result<Uuid, DomainError>;
    fn get_family_properties(&self, family_id: &Uuid) -> Result<Vec<OwnedFamilyProperty>, DomainError>;
    fn save_molecular_property(&self, prop: OwnedMolecularProperty) -> Result<Uuid, DomainError>;
    fn get_molecular_properties(&self, inchikey: &str) -> Result<Vec<OwnedMolecularProperty>, DomainError>;
}
```

**Beneficios**:

- Composición flexible: `impl MoleculeReader + MoleculeWriter + FamilyRepository`
- Mocks simples: solo implementar lo necesario
- Cambios aislados: modificar propiedades no afecta moléculas

---

### 2. `StepContext` - Violación de SRP (Single Responsibility)

**Ubicación**: `crates/chem-workflow/src/step/context.rs:16-88`

**Problema**: Struct con 3 repositorios diferentes y múltiples responsabilidades (persistencia, lectura, deduplicación)

**Código actual**:

```rust
pub struct StepContext {
    pub flow_id: Uuid,
    pub flow_repo: Arc<dyn FlowRepository>,
    pub domain_repo: Arc<dyn DomainRepository>,
}

impl StepContext {
    // Lectura de payloads (concern 1)
    pub fn get_step_payload_by_name(&self, step_name: &str) -> Result<Option<JsonValue>> { /* ... */ }
    pub fn get_typed_output_by_type<T>(&self) -> Result<Option<T>> { /* ... */ }
    pub fn get_step_payload_by_name_typed<T>(&self, step_name: &str) -> Result<Option<T>> { /* ... */ }

    // Persistencia de payloads (concern 2)
    pub fn save_typed_result(&self, step_name: &str, info: StepInfo, expected_version: i64, command_id: Option<Uuid>) -> Result<PersistResult> {
        // Deduplicación (concern 3)
        let existing = self.flow_repo.read_data(&self.flow_id, 0)?;
        if existing.iter().any(|fd| fd.key.eq_ignore_ascii_case(&key) && fd.payload == info.payload) {
            // ...
        }
        // Determinación de cursor/versión (concern 4)
        let (cursor_candidate, ev) = self.flow_repo.get_flow_meta(&self.flow_id).map(|meta| { /* ... */ })?;
        // Persistencia real (concern 5)
        self.flow_repo.persist_data(&data, ev)?
    }
}
```

**Impacto**:

- 5 razones para cambiar (SRP violado)
- Testing complejo (requiere 2 repositorios mockeados)
- Lógica de negocio (deduplicación) mezclada con infraestructura

**Refactorización**:

```rust
// Separar en contextos especializados

pub struct ReadContext<R>
where R: MoleculeReader + FamilyRepository
{
    flow_id: Uuid,
    flow_repo: Arc<dyn FlowRepository>,
    domain_repo: Arc<R>,
}

impl<R> ReadContext<R>
where R: MoleculeReader + FamilyRepository
{
    pub fn get_molecule(&self, inchikey: &str) -> Result<Option<Molecule>> {
        self.domain_repo.get_molecule(inchikey).map_err(Into::into)
    }

    pub fn get_step_payload<T>(&self, step_name: &str) -> Result<Option<T>>
    where T: DeserializeOwned
    {
        let key = key_for_step_state(step_name);
        let data = self.flow_repo.read_data(&self.flow_id, 0)?;
        for fd in data.iter().rev() {
            if fd.key.eq_ignore_ascii_case(&key) {
                return Ok(serde_json::from_value(fd.payload.clone()).ok());
            }
        }
        Ok(None)
    }
}

pub struct WriteContext<W>
where W: MoleculeWriter + FamilyRepository
{
    flow_id: Uuid,
    flow_repo: Arc<dyn FlowRepository>,
    domain_repo: Arc<W>,
}

impl<W> WriteContext<W>
where W: MoleculeWriter + FamilyRepository
{
    pub fn save_molecule(&self, molecule: Molecule) -> Result<String> {
        self.domain_repo.save_molecule(molecule).map_err(Into::into)
    }

    pub fn persist_step(&self, step_name: &str, payload: JsonValue) -> Result<PersistResult> {
        // Solo persistencia, dedup en servicio separado
        let key = key_for_step_state(step_name);
        let meta = self.flow_repo.get_flow_meta(&self.flow_id)?;
        let data = FlowData {
            id: Uuid::new_v4(),
            flow_id: self.flow_id,
            cursor: meta.current_cursor + 1,
            key,
            payload,
            metadata: json!({}),
            command_id: None,
            created_at: Utc::now(),
        };
        self.flow_repo.persist_data(&data, meta.current_version).map_err(Into::into)
    }
}

// Servicio de dominio para deduplicación
pub struct DeduplicationService;

impl DeduplicationService {
    pub fn is_duplicate(&self, flow_repo: &dyn FlowRepository, flow_id: &Uuid, key: &str, payload: &JsonValue) -> Result<bool> {
        let existing = flow_repo.read_data(flow_id, 0)?;
        Ok(existing.iter().any(|fd| fd.key.eq_ignore_ascii_case(key) && &fd.payload == payload))
    }
}
```

**Beneficios**:

- SRP cumplido: cada contexto tiene una sola razón para cambiar
- Testing simple: mockear solo lo necesario
- Lógica de negocio (dedup) en servicio de dominio

---

### 3. `chem-persistence` - Violación de DIP (Dependency Inversion)

**Ubicación**: `crates/chem-persistence/src/domain_persistence.rs:112-135`

**Problema**: Lógica de negocio en adapter, violando flujo de dependencias (dominio → adapter)

**Código actual**:

```rust
impl DomainRepository for DieselDomainRepository {
    fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError> {
        let mut conn = self.conn()?;

        // ❌ Lógica de negocio EN EL ADAPTER (debería estar en dominio)
        let families = self.list_families()?;
        for fam in families {
            if fam.contains(inchikey) {
                return Err(DomainError::ValidationError(
                    format!("No se puede eliminar la molécula {}; pertenece a una familia", inchikey)
                ));
            }
        }

        // Persistencia
        diesel::delete(molecules.filter(inchikey_col.eq(inchikey)))
            .execute(&mut conn)
            .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
        Ok(())
    }
}
```

**Impacto**:

- Dominio depende indirectamente del adapter (violación DIP)
- Lógica de negocio duplicada si agregamos otro adapter (SQLx)
- Testing incompleto (lógica no testeada en dominio)

**Refactorización**:

```rust
// 1. Crear servicio de dominio
// En crates/chem-domain/src/services/molecule_service.rs
pub struct MoleculeService<R, W>
where
    R: MoleculeReader + FamilyRepository,
    W: MoleculeWriter,
{
    reader: Arc<R>,
    writer: Arc<W>,
}

impl<R, W> MoleculeService<R, W>
where
    R: MoleculeReader + FamilyRepository,
    W: MoleculeWriter,
{
    pub fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError> {
        // Lógica de negocio EN EL DOMINIO
        let families = self.reader.list_families()?;
        for fam in families {
            if fam.contains(inchikey) {
                return Err(DomainError::ValidationError(
                    format!("Cannot delete molecule {}; belongs to family {}", inchikey, fam.id())
                ));
            }
        }

        // Delegar a writer (adapter)
        self.writer.delete_molecule(inchikey)
    }
}

// 2. Adapter solo hace persistencia
impl MoleculeWriter for DieselMoleculeRepository {
    fn delete_molecule(&self, inchikey: &str) -> Result<(), DomainError> {
        let mut conn = self.conn()?;
        diesel::delete(molecules.filter(inchikey_col.eq(inchikey)))
            .execute(&mut conn)
            .map_err(|e| DomainError::ExternalError(format!("db: {}", e)))?;
        Ok(())
    }
}
```

**Beneficios**:

- DIP cumplido: dominio define reglas, adapters las ejecutan
- Lógica de negocio testeable sin DB
- Reutilizable con cualquier adapter (SQLx, in-memory, etc.)

---

### 4. `chem-providers` - Violación de OCP + DIP (Open-Closed + Dependency Inversion)

**Ubicación**: `crates/chem-providers/src/core.rs:15-88`

**Problema**: No existe trait/port; funciones directas a subprocess; cerrado a extensión

**Código actual**:

```rust
// ❌ Funciones libres, no abstracción
pub fn get_molecule(smiles: &str) -> Result<Molecule, anyhow::Error> {
    // Hardcoded subprocess call
    let output = Command::new("python3")
        .arg("python/rdkit_wrapper.py")
        .arg("get_molecule")
        .arg(smiles)
        .output()?;
    // ...
}

pub fn calculate_properties(smiles: &str, props: Vec<String>) -> Result<HashMap<String, f64>, anyhow::Error> {
    // Hardcoded subprocess call
    let output = Command::new("python3")
        .arg("python/rdkit_wrapper.py")
        .arg("calculate_properties")
        .arg(smiles)
        .arg(&props.join(","))
        .output()?;
    // ...
}
```

**Impacto**:

- Imposible mockear para tests
- Cerrado a extensión (no se puede agregar ChemAxon sin modificar código)
- Workflow acoplado a implementación concreta

**Refactorización**:

```rust
// 1. Definir port en dominio
// En crates/chem-domain/src/ports/property_provider.rs
pub trait PropertyProvider: Send + Sync {
    fn calculate_properties(
        &self,
        smiles: &str,
        properties: &[PropertyType]
    ) -> Result<HashMap<PropertyType, f64>, DomainError>;

    fn generate_structure(&self, smiles: &str) -> Result<MoleculeStructure, DomainError>;
    fn validate_smiles(&self, smiles: &str) -> Result<bool, DomainError>;
}

// 2. Implementar adapter RDKit
// En crates/chem-providers/src/adapters/rdkit_property_provider.rs
pub struct RDKitPropertyProvider {
    python_path: String,
}

impl PropertyProvider for RDKitPropertyProvider {
    fn calculate_properties(
        &self,
        smiles: &str,
        properties: &[PropertyType]
    ) -> Result<HashMap<PropertyType, f64>, DomainError> {
        let output = Command::new(&self.python_path)
            .arg("python/rdkit_wrapper.py")
            .arg("calculate_properties")
            .arg(smiles)
            .arg(&Self::serialize_props(properties))
            .output()
            .map_err(|e| DomainError::ExternalProvider(e.into()))?;

        Self::parse_output(&output)
    }
}

// 3. Implementar mock para tests
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
        results.get(smiles)
            .cloned()
            .ok_or_else(|| DomainError::NotFound {
                entity_type: "PropertyResult".into(),
                id: smiles.into()
            })
    }
}
```

**Beneficios**:

- OCP cumplido: extensible sin modificar código existente
- DIP cumplido: workflow depende de trait, no de impl
- Testing simple con mocks

---

### 5. `chem-workflow` - Violación de DIP (Dependency Inversion)

**Ubicación**: `crates/chem-workflow/src/flows/cadma_flow/mod.rs:25-47` y tests

**Problema**: Workflow construye impls concretas directamente

**Código actual**:

```rust
// En tests
let flow_repo = Arc::new(InMemoryFlowRepository::new());  // ❌ Impl concreta
let domain_repo = Arc::new(InMemoryDomainRepository::new());  // ❌ Impl concreta
let ctx = StepContext::new(flow_id, flow_repo, domain_repo);

// En workflow
impl CadmaFlow {
    pub fn new(
        flow_repo: Arc<dyn FlowRepository>,
        domain_repo: Arc<dyn DomainRepository>,  // ❌ Trait muy amplio (ISP violado)
    ) -> Self {
        // ...
    }
}
```

**Impacto**:

- Tests acoplados a impls concretas
- No se puede inyectar adapters reales fácilmente
- Dificulta testing de integración

**Refactorización**:

```rust
// 1. Workflow acepta ports, no impls
pub struct CadmaFlow<R, W, P>
where
    R: MoleculeReader + FamilyRepository,
    W: MoleculeWriter + FamilyRepository,
    P: PropertyProvider,
{
    molecule_reader: Arc<R>,
    molecule_writer: Arc<W>,
    property_provider: Arc<P>,
    flow_repo: Arc<dyn FlowRepository>,
}

impl<R, W, P> CadmaFlow<R, W, P>
where
    R: MoleculeReader + FamilyRepository,
    W: MoleculeWriter + FamilyRepository,
    P: PropertyProvider,
{
    pub fn new(
        molecule_reader: Arc<R>,
        molecule_writer: Arc<W>,
        property_provider: Arc<P>,
        flow_repo: Arc<dyn FlowRepository>,
    ) -> Self {
        Self { molecule_reader, molecule_writer, property_provider, flow_repo }
    }

    pub fn run(&self, flow_id: Uuid) -> Result<()> {
        let read_ctx = ReadContext::new(self.flow_repo.clone(), self.molecule_reader.clone());
        let write_ctx = WriteContext::new(self.flow_repo.clone(), self.molecule_writer.clone());
        let prop_ctx = PropertyContext::new(self.property_provider.clone());

        // Ejecutar steps con contextos especializados
        // ...
    }
}

// 2. Tests con mocks
#[test]
fn cadma_flow_validates_family() {
    let mock_reader = Arc::new(MockMoleculeReader::new());
    let mock_writer = Arc::new(MockMoleculeWriter::new());
    let mock_provider = Arc::new(MockPropertyProvider::new());
    let flow_repo = Arc::new(InMemoryFlowRepository::new());

    let cadma = CadmaFlow::new(mock_reader, mock_writer, mock_provider, flow_repo);
    // ...
}
```

**Beneficios**:

- DIP cumplido: depende de abstracciones
- Tests con mocks simples
- Fácil reemplazar impls en producción

---

## Violaciones Altas

### 6. Duplicación de Engine Logic - Violación de DRY

**Ubicación**:

- `crates/chem-workflow/src/engine/mod.rs`
- `crates/flow/src/engine.rs`

**Problema**: Lógica de engine duplicada en dos crates

**Refactorización**: Consolidar en `flow/src/engine.rs` y que `chem-workflow` use composición

---

### 7. `FlowRepository` - Violación menor de ISP

**Ubicación**: `crates/flow/src/repository.rs:37-124`

**Problema**: Trait con 15 métodos; algunos clientes no usan todos

**Refactorización**: Separar en `FlowReader`, `FlowWriter`, `FlowBranchManager`

---

### 8. `WorkflowFactory` poco usado - Violación de OCP

**Ubicación**: `crates/chem-workflow/src/factory/workflow_factory.rs`

**Problema**: Tests construyen workflows manualmente; factory existe pero no se usa

**Refactorización**: Usar factory consistentemente + inyección de dependencias

---

### 9. Lógica de persistencia en `save_typed_result` - Violación de SRP

**Ubicación**: `crates/chem-workflow/src/step/context.rs:52-75`

**Problema**: Método con 4 responsabilidades (dedup, versión, cursor, persistencia)

**Refactorización**: Ya cubierto en violación #2

---

### 10. `DieselFlowRepository` - Violación menor de SRP

**Ubicación**: `crates/chem-persistence/src/flow_persistence.rs`

**Problema**: Clase con 500+ líneas implementando múltiples concerns

**Refactorización**: Separar en `DieselFlowReader`, `DieselFlowWriter`, `DieselBranchManager`

---

### 11. Errores genéricos en `chem-providers` - Violación de claridad

**Ubicación**: `crates/chem-providers/src/core.rs`

**Problema**: Uso de `anyhow::Error` en lugar de errores tipados

**Refactorización**: Crear `ProvidersError` enum

---

### 12. `main.rs` sin inyección de dependencias - Violación de DIP

**Ubicación**: `src/main.rs`

**Problema**: No existe app container; todo hardcoded

**Refactorización**: Ya cubierto en arquitectura objetivo

---

## Violaciones Medias

### 13. Tests sin helpers compartidos - Violación de DRY

**Ubicación**: Tests en múltiples crates

**Problema**: Código de setup duplicado

**Refactorización**: Expandir `chem-utils/src/test_helpers/`

---

### 14. `StepInfo` como struct genérico - Violación de claridad

**Ubicación**: `crates/chem-workflow/src/step/trait_step.rs:13-17`

**Problema**: Struct con `JsonValue` para todo

**Refactorización**: Usar tipos específicos por step

---

### 15. Falta de validación en constructores - Violación de invariantes

**Ubicación**: Múltiples entidades

**Problema**: Constructores no validan (e.g., `Molecule::from_parts`)

**Refactorización**: Usar builder pattern con validación

---

## Resumen por Principio

| Principio | Violaciones | Severidad Promedio |
| --------- | ----------- | ------------------ |
| S (SRP)   | 5           | Alta               |
| O (OCP)   | 3           | Media              |
| L (LSP)   | 0           | -                  |
| I (ISP)   | 3           | Crítica            |
| D (DIP)   | 4           | Crítica            |

## Priorización de Refactoring

1. **Fase 2** (Crítico): Violaciones #1, #2, #3 (ISP, SRP, DIP en dominio)
2. **Fase 3-4** (Alto): Violaciones #4, #5 (adapters + DIP en workflow)
3. **Fase 5-6** (Medio): Violaciones #6-12 (consolidación + DI)
4. **Fase 7** (Bajo): Violaciones #13-15 (polish + tests)

## Conclusión

El proyecto tiene violaciones SOLID moderadas pero sistemáticas. La refactorización hexagonal las resolverá naturalmente al:

1. Separar ports (traits pequeños) → ISP cumplido
2. Crear servicios de dominio → SRP cumplido
3. Inyección de dependencias → DIP cumplido
4. Adapters como extensiones → OCP cumplido

**Esfuerzo estimado**: 12 días (ver `REFACTOR_PLAN.md`)
