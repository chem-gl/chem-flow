# Flow-Chem

**Flow-Chem** es una plataforma modular en Rust para modelar, versionar y persistir flujos de trabajo químicos (molecular families, propiedades calculadas y workflows complejos como CADMA). Diseñada con arquitectura hexagonal, sigue principios SOLID y permite integración con motores químicos (RDKit) y múltiples backends de persistencia (SQLite/Postgres).

---

## 📋 Tabla de Contenidos

- [Visión General](#-visión-general)
- [Arquitectura del Sistema](#-arquitectura-del-sistema)
  - [Arquitectura Hexagonal](#arquitectura-hexagonal)
  - [Diagrama de Alto Nivel](#diagrama-de-alto-nivel)
  - [Diagrama de Clases del Dominio](#diagrama-de-clases-del-dominio)
  - [Diagrama de Secuencia: Ejecución de Workflow](#diagrama-de-secuencia-ejecución-de-workflow)
- [Estructura del Workspace](#-estructura-del-workspace)
- [Descripción de Crates](#-descripción-de-crates)
- [Requisitos del Sistema](#-requisitos-del-sistema)
- [Instalación y Configuración](#-instalación-y-configuración)
- [Ejecución del Proyecto](#-ejecución-del-proyecto)
  - [Con Docker (Recomendado)](#con-docker-recomendado)
  - [Localmente](#localmente)
- [Ejecución de Examples](#-ejecución-de-examples)
- [Testing](#-testing)
- [Calidad de Código](#-calidad-de-código)
- [Variables de Entorno](#-variables-de-entorno)
- [Flujos de Trabajo Principales](#-flujos-de-trabajo-principales)
- [Principios de Diseño](#-principios-de-diseño)
- [Contribución](#-contribución)
- [Licencia](#-licencia)

---

## 🎯 Visión General

**Flow-Chem** permite:

- **Modelado de entidades químicas**: Moléculas, familias moleculares y propiedades con verificación de integridad mediante hashing.
- **Versionado de flujos**: Persistencia de workflows con snapshots, branching y replay.
- **Integración con motores químicos**: Uso de RDKit (vía PyO3) para cálculos y generación de estructuras.
- **Persistencia flexible**: SQLite (desarrollo/tests) y PostgreSQL (producción).
- **Workflows complejos**: Implementación de CADMA (Computer-Aided Drug Molecular Architecture) con múltiples pasos de análisis y generación.

---

## 🏗 Arquitectura del Sistema

### Arquitectura Hexagonal

El proyecto sigue **Arquitectura Hexagonal (Ports & Adapters)**, separando el dominio puro de las implementaciones de infraestructura:

```mermaid
graph TB
    subgraph "Application Layer"
        Examples[Examples & CLI]
        Workflows[chem-workflow]
    end
    
    subgraph "Domain Layer (Core)"
        Domain[chem-domain]
        Flow[flow]
    end
    
    subgraph "Infrastructure Layer (Adapters)"
        Persistence[chem-persistence<br/>Diesel + r2d2]
        Providers[chem-providers<br/>RDKit via PyO3]
    end
    
    subgraph "External Systems"
        DB[(PostgreSQL/SQLite)]
        Python[Python + RDKit]
    end
    
    Examples --> Workflows
    Workflows --> Domain
    Workflows --> Flow
    Domain -.->|Ports| Persistence
    Domain -.->|Ports| Providers
    Flow -.->|Ports| Persistence
    Persistence --> DB
    Providers --> Python
    
    style Domain fill:#e1f5ff
    style Flow fill:#e1f5ff
    style Persistence fill:#fff4e1
    style Providers fill:#fff4e1
```

**Capas:**

1. **Dominio (Core)**: Lógica de negocio pura sin dependencias externas
   - `chem-domain`: Entidades químicas (Molecule, MoleculeFamily, Properties)
   - `flow`: Gestión de flujos versionados (FlowData, FlowRepository)

2. **Puertos (Interfaces)**: Traits que definen contratos
   - `DomainRepository`, `PropertyProvider`, `ChemEngineInterface`
   - `FlowRepository`, `SnapshotStore`

3. **Adaptadores (Infraestructura)**: Implementaciones concretas
   - `chem-persistence`: Repositorios Diesel (SQLite/Postgres)
   - `chem-providers`: Integración con RDKit via PyO3

4. **Aplicación**: Orquestación y casos de uso
   - `chem-workflow`: Workflows CADMA con steps composables
   - `examples`: Demos y CLIs interactivos

---

### Diagrama de Alto Nivel

```mermaid
flowchart TB
    subgraph Client["🖥 Client Layer"]
        CLI[CLI Examples]
        API[Future API]
    end
    
    subgraph Application["📦 Application Layer"]
        WorkflowEngine[ChemicalWorkflowEngine]
        CadmaFlow[CadmaFlow]
        Steps[Steps 1-6]
    end
    
    subgraph Domain["💎 Domain Layer"]
        Molecule[Molecule]
        Family[MoleculeFamily]
        Properties[Properties]
        FlowEngine[FlowEngine]
        FlowData[FlowData]
    end
    
    subgraph Ports["🔌 Ports/Interfaces"]
        DomainRepo[DomainRepository]
        FlowRepo[FlowRepository]
        PropProvider[PropertyProvider]
        ChemEngine[ChemEngineInterface]
    end
    
    subgraph Adapters["🔧 Adapters"]
        DieselDomain[DieselDomainRepository]
        DieselFlow[DieselFlowRepository]
        RDKitProvider[RDKit Provider]
    end
    
    subgraph External["🌐 External Systems"]
        DB[(PostgreSQL)]
        RDKit[Python RDKit]
    end
    
    CLI --> WorkflowEngine
    API -.-> WorkflowEngine
    WorkflowEngine --> CadmaFlow
    CadmaFlow --> Steps
    Steps --> Molecule
    Steps --> Family
    Steps --> FlowEngine
    
    Molecule --> DomainRepo
    Family --> DomainRepo
    FlowEngine --> FlowRepo
    Molecule --> PropProvider
    Molecule --> ChemEngine
    
    DomainRepo -.->|implements| DieselDomain
    FlowRepo -.->|implements| DieselFlow
    PropProvider -.->|implements| RDKitProvider
    ChemEngine -.->|implements| RDKitProvider
    
    DieselDomain --> DB
    DieselFlow --> DB
    RDKitProvider --> RDKit
    
    style Domain fill:#e3f2fd
    style Ports fill:#fff3e0
    style Adapters fill:#f3e5f5
    style External fill:#e8f5e9
```

---

### Diagrama de Clases del Dominio

```mermaid
classDiagram
    %% Domain Entities
    class Molecule {
        -String inchikey
        -String smiles
        -String inchi
        -JsonValue metadata
        +from_parts(inchikey, smiles, inchi, metadata) Result~Molecule~
        +from_smiles(smiles) Result~Molecule~
        +from_provider_molecule(smiles, provider) Result~Molecule~
        +inchikey() &str
        +smiles() &str
        +inchi() &str
        +metadata() &JsonValue
    }
    
    class MoleculeFamily {
        -Uuid id
        -Option~String~ name
        -Option~String~ description
        -String family_hash
        -Vec~Molecule~ molecules
        -JsonValue provenance
        -bool frozen
        +new(molecules, provenance) Result~MoleculeFamily~
        +add_molecule(molecule) Result~MoleculeFamily~
        +remove_molecule(inchikey) Result~MoleculeFamily~
        +with_name(name) MoleculeFamily
        +with_description(desc) MoleculeFamily
        +verify_integrity() bool
        +id() Uuid
        +molecules() &Vec~Molecule~
        +family_hash() &str
    }
    
    class MolecularProperty {
        -Uuid id
        -String molecule_inchikey
        -String property_type
        -JsonValue value
        -Option~String~ quality
        -bool preferred
        -String value_hash
        -JsonValue metadata
        +new(...) Result~MolecularProperty~
        +verify_integrity() Result~bool~
    }
    
    class FamilyProperty {
        -Uuid id
        -Uuid family_id
        -String property_type
        -JsonValue value
        -Option~String~ quality
        -bool preferred
        -String value_hash
        -JsonValue metadata
        +new(...) Result~FamilyProperty~
        +verify_integrity() Result~bool~
    }
    
    %% Ports (Traits)
    class DomainRepository {
        <<interface>>
        +save_molecule(m: Molecule) Result~String~
        +get_molecule(inchikey: &str) Result~Option~Molecule~~
        +list_molecules() Result~Vec~Molecule~~
        +delete_molecule(inchikey: &str) Result~()~
        +save_family(f: MoleculeFamily) Result~Uuid~
        +get_family(id: &Uuid) Result~Option~MoleculeFamily~~
        +list_families() Result~Vec~MoleculeFamily~~
        +delete_family(id: &Uuid) Result~()~
        +save_molecular_property(prop: OwnedMolecularProperty) Result~Uuid~
        +get_molecular_properties(inchikey: &str) Result~Vec~OwnedMolecularProperty~~
        +save_family_property(prop: OwnedFamilyProperty) Result~Uuid~
        +get_family_properties(family_id: &Uuid) Result~Vec~OwnedFamilyProperty~~
        +add_molecule_to_family(family_id: &Uuid, molecule: Molecule) Result~Uuid~
        +remove_molecule_from_family(family_id: &Uuid, inchikey: &str) Result~Uuid~
    }
    
    class PropertyProvider {
        <<interface>>
        +calculate_property(molecule: &Molecule, property_type: &str) Result~JsonValue~
        +calculate_batch(molecules: &[Molecule], properties: &[String]) Result~HashMap~
    }
    
    class ChemEngineInterface {
        <<interface>>
        +get_molecule(smiles: &str) Result~ProviderMolecule~
        +generate_substitutions(...) Result~Vec~String~~
        +calculate_properties(...) Result~HashMap~
    }
    
    %% Flow Domain
    class FlowData {
        -Uuid id
        -Uuid flow_id
        -i64 cursor
        -String key
        -JsonValue payload
        -JsonValue metadata
        -Option~Uuid~ command_id
        -DateTime created_at
    }
    
    class FlowMeta {
        -Uuid id
        -Option~String~ name
        -Option~String~ status
        -Option~Uuid~ parent_flow_id
        -Option~i64~ parent_cursor
        -i64 current_cursor
        -i64 current_version
        -JsonValue metadata
    }
    
    class FlowRepository {
        <<interface>>
        +create_flow(name, status, metadata) Result~Uuid~
        +persist_data(data: FlowData, expected_version: i64) Result~PersistResult~
        +read_data(flow_id: &Uuid, from_cursor: i64) Result~Vec~FlowData~~
        +create_branch(parent_id, parent_cursor, metadata) Result~Uuid~
        +delete_branch(flow_id: &Uuid) Result~()~
        +get_flow_meta(flow_id: &Uuid) Result~FlowMeta~
        +save_snapshot(flow_id, cursor, data, metadata) Result~Uuid~
        +load_snapshot(snapshot_id: &Uuid) Result~(Vec~u8~, JsonValue)~
    }
    
    %% Adapters
    class DieselDomainRepository {
        -DbPool pool
        +new(database_url: &str) Result~Self~
    }
    
    class DieselFlowRepository {
        -DbPool pool
        +new(database_url: &str) Result~Self~
    }
    
    class ChemEngine {
        -PyObject rdkit_module
        +init() Result~Self~
    }
    
    %% Relationships
    MoleculeFamily "1" *-- "*" Molecule : contains
    MolecularProperty --> Molecule : describes
    FamilyProperty --> MoleculeFamily : describes
    
    DomainRepository <|.. DieselDomainRepository : implements
    FlowRepository <|.. DieselFlowRepository : implements
    PropertyProvider <|.. ChemEngine : implements
    ChemEngineInterface <|.. ChemEngine : implements
    
    DomainRepository ..> Molecule : persists
    DomainRepository ..> MoleculeFamily : persists
    DomainRepository ..> MolecularProperty : persists
    DomainRepository ..> FamilyProperty : persists
    
    FlowRepository ..> FlowData : persists
    FlowRepository ..> FlowMeta : manages
```

---

### Diagrama de Secuencia: Ejecución de Workflow

```mermaid
sequenceDiagram
    actor User
    participant CLI as CLI/Example
    participant Engine as ChemicalWorkflowEngine
    participant Step as WorkflowStep
    participant Domain as Domain Entities
    participant FlowRepo as FlowRepository
    participant DomainRepo as DomainRepository
    participant ChemEngine as ChemEngine (RDKit)
    participant DB as Database
    
    User->>CLI: Ejecutar workflow CADMA
    CLI->>Engine: create_flow(name)
    Engine->>FlowRepo: create_flow(...)
    FlowRepo->>DB: INSERT INTO flows
    DB-->>FlowRepo: flow_id
    FlowRepo-->>Engine: flow_id
    
    Note over Engine: Estado inicial creado
    
    User->>CLI: Ejecutar Step1 (Family)
    CLI->>Engine: execute_step(0, input)
    Engine->>Step: Step1::execute(input, context)
    Step->>DomainRepo: get_family(id) or create
    DomainRepo->>DB: SELECT families, molecules
    DB-->>DomainRepo: Family data
    DomainRepo-->>Step: MoleculeFamily
    Step->>Domain: MoleculeFamily::new(...)
    Domain-->>Step: family (with hash)
    Step->>DomainRepo: save_family(family)
    DomainRepo->>DB: INSERT families, family_members
    DB-->>DomainRepo: family_id
    Step-->>Engine: StepResult (payload)
    Engine->>FlowRepo: persist_data(flow_data, version)
    FlowRepo->>DB: INSERT flow_data, UPDATE flows
    DB-->>FlowRepo: OK
    
    Note over Engine: Step1 completado
    
    User->>CLI: Ejecutar Step2 (ADMETSA)
    CLI->>Engine: execute_step(1, input)
    Engine->>Step: Step2::execute(input, context)
    Step->>DomainRepo: get_molecules(family)
    DomainRepo->>DB: SELECT molecules
    DB-->>DomainRepo: molecules
    Step->>ChemEngine: calculate_properties(molecules, props)
    ChemEngine->>ChemEngine: RDKit calculations
    ChemEngine-->>Step: PropertyValues
    Step->>DomainRepo: save_molecular_property(prop)
    DomainRepo->>DB: INSERT molecular_properties
    DB-->>DomainRepo: property_id
    Step-->>Engine: StepResult
    Engine->>FlowRepo: persist_data(...)
    FlowRepo->>DB: INSERT flow_data, UPDATE flows
    
    Note over Engine: Step2 completado
    
    User->>CLI: Crear branch desde cursor 2
    CLI->>Engine: create_branch(parent_id, cursor)
    Engine->>FlowRepo: create_branch(...)
    FlowRepo->>DB: BEGIN TRANSACTION
    FlowRepo->>DB: INSERT new flow (parent ref)
    FlowRepo->>DB: COPY flow_data WHERE cursor <= 2
    FlowRepo->>DB: COPY snapshots WHERE cursor <= 2
    FlowRepo->>DB: COMMIT
    DB-->>FlowRepo: branch_id
    FlowRepo-->>Engine: branch_id
    Engine-->>CLI: branch_id
    
    Note over Engine: Branch creado desde paso 2
```

---

## 📁 Estructura del Workspace

```
flow-chem/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── README.md                     # Este archivo
├── docker-compose.yml            # Servicios (Postgres, app-dev)
├── Dockerfile                    # Imagen de desarrollo
├── rust-toolchain               # Versión de Rust
├── sonar-project.properties     # Configuración SonarQube
│
├── crates/
│   ├── chem-domain/             # 💎 Dominio químico puro
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── molecule.rs
│   │   │   ├── molecule_family.rs
│   │   │   ├── molecular_property.rs
│   │   │   ├── family_property.rs
│   │   │   ├── errors.rs
│   │   │   ├── ports/          # Traits/Interfaces
│   │   │   │   ├── mod.rs
│   │   │   │   ├── molecule_reader.rs
│   │   │   │   ├── molecule_writer.rs
│   │   │   │   ├── family_repository.rs
│   │   │   │   ├── property_repository.rs
│   │   │   │   └── property_provider.rs
│   │   │   ├── services/       # Domain services
│   │   │   │   ├── molecule_service.rs
│   │   │   │   └── family_service.rs
│   │   │   └── application/    # Use cases
│   │   │       └── use_cases.rs
│   │   └── tests/
│   │
│   ├── flow/                    # 🔄 Gestión de flujos versionados
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── domain.rs       # FlowData, FlowMeta
│   │   │   ├── repository.rs   # FlowRepository trait
│   │   │   ├── engine.rs       # FlowEngine helper
│   │   │   ├── stubs.rs        # InMemory impl
│   │   │   └── errors.rs
│   │   └── tests/
│   │
│   ├── chem-persistence/        # 🗄 Adaptadores Diesel
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── db.rs           # Pool setup
│   │   │   ├── schema.rs       # Diesel schema
│   │   │   ├── domain_persistence.rs  # DieselDomainRepository
│   │   │   ├── flow_persistence.rs    # DieselFlowRepository
│   │   │   ├── migrations.rs
│   │   │   └── test_helpers.rs
│   │   ├── migrations/         # SQL migrations
│   │   └── tests/
│   │
│   ├── chem-providers/          # 🧪 Integración RDKit (PyO3)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── core.rs         # ChemEngine
│   │   │   └── test_utils.rs   # Mocks
│   │   └── python/
│   │       └── rdkit_wrapper.py
│   │
│   ├── chem-workflow/           # 📋 Workflows CADMA
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── workflow_type.rs
│   │   │   ├── errors.rs
│   │   │   ├── engine/         # ChemicalWorkflowEngine
│   │   │   ├── factory/        # Factory pattern
│   │   │   ├── flows/          # CadmaFlow
│   │   │   │   └── cadma_flow/
│   │   │   │       └── steps/  # Step1-Step6
│   │   │   └── step/           # Base step trait
│   │   ├── examples/
│   │   │   └── cadma_example.rs
│   │   └── tests/
│   │
│   └── chem-utils/              # 🛠 Utilidades y helpers
│       └── src/
│
├── examples/                    # 📝 Ejemplos standalone
│   ├── example-domain.rs       # CLI interactivo dominio
│   └── example-main.rs         # CLI interactivo flows
│
├── scripts/                     # 🔧 Scripts de automatización
│   ├── run_examples.sh         # Ejecutar examples (con menú)
│   ├── run_tests_in_docker.sh
│   ├── run_tests_with_mocks.sh
│   └── generate_coverage.sh
│
└── target/                      # Build artifacts (gitignored)
```

---

## 📦 Descripción de Crates

### `chem-domain` 💎

**Responsabilidad**: Núcleo del dominio químico. Define entidades, reglas de negocio y ports (interfaces).

**Contenido clave:**
- `Molecule`: Representación inmutable de moléculas con validación de InChIKey, SMILES e InChI
- `MoleculeFamily`: Colección versionada de moléculas con `family_hash` para integridad
- `MolecularProperty` / `FamilyProperty`: Propiedades calculadas con `value_hash`
- **Ports**: `DomainRepository`, `PropertyProvider`, `MoleculeReader`, `MoleculeWriter`
- **Services**: Lógica de dominio compleja (creación de familias, validaciones)

**Principios:**
- Sin dependencias de infraestructura
- Inmutabilidad: operaciones devuelven nuevas instancias
- Validación estricta en constructores

---

### `flow` 🔄

**Responsabilidad**: Gestión de flujos versionados con persistencia por eventos.

**Contenido clave:**
- `FlowData`: Registro de paso de workflow (cursor, payload, metadata)
- `FlowMeta`: Metadatos del flujo (current_cursor, current_version, parent)
- `FlowRepository` trait: Contrato de persistencia
- `FlowEngine`: Helper para operaciones comunes (append, branch, snapshot)
- `InMemoryFlowRepository`: Implementación en memoria para tests

**Características:**
- **Versionado optimista**: `expected_version` previene conflictos de escritura
- **Branching**: Crear ramas desde cualquier cursor
- **Snapshots**: Guardado de estado para rehidratación rápida
- **Replay**: Reconstruir estado desde eventos (flow_data)

---

### `chem-persistence` 🗄

**Responsabilidad**: Adaptadores de persistencia usando Diesel ORM.

**Contenido clave:**
- `DieselDomainRepository`: Implementa `DomainRepository` para SQLite/Postgres
- `DieselFlowRepository`: Implementa `FlowRepository`
- **Schema Diesel**: Tablas `flows`, `flow_data`, `snapshots`, `molecules`, `families`, etc.
- **Migraciones**: SQL embebido para setup automático
- **Pool de conexiones**: r2d2 para concurrencia

**Features:**
- `sqlite` (default): Tests y desarrollo local
- `postgres` / `pg`: Producción con PostgreSQL

---

### `chem-providers` 🧪

**Responsabilidad**: Integración con motores químicos (RDKit via PyO3).

**Contenido clave:**
- `ChemEngine`: Wrapper de RDKit Python
- `ChemEngineInterface` trait: Contrato para providers
- **Operaciones**: 
  - Conversión SMILES → InChI/InChIKey
  - Cálculo de propiedades (LogP, PSA, etc.)
  - Generación de sustituciones moleculares
- **Mock**: Implementación fake para tests sin RDKit

**Features:**
- `mock_rdkit`: Usa mocks en lugar de RDKit real

---

### `chem-workflow` 📋

**Responsabilidad**: Definición de workflows complejos (CADMA).

**Contenido clave:**
- `ChemicalWorkflowEngine` trait: Base para workflows
- `CadmaFlow`: Workflow CADMA con 6 steps
  - **Step1**: Referencia de familia inicial
  - **Step2**: Cálculo ADMETSA (propiedades)
  - **Step3**: Generación de molécula inicial
  - **Step4**: ADMETSA para molécula inicial
  - **Step5**: Generación de sustituciones
  - **Step6**: ADMETSA para moléculas generadas
- `WorkflowFactory`: Creación y carga de workflows
- **Steps composables**: Cada step es independiente y testeable

**Características:**
- Integración con `flow` para persistencia
- Inyección de `DomainRepository` y `ChemEngine`
- Validación de dependencias entre steps
- Metadata y provenance tracking

---

### `chem-utils` 🛠

**Responsabilidad**: Utilidades compartidas y helpers para testing.

**Contenido**: Funciones de test, fixtures, macros comunes.

---

## 💻 Requisitos del Sistema

### Software

- **Rust**: 1.70+ (edition 2021)
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

- **Docker** y **docker-compose**: Para entorno de desarrollo con RDKit y Postgres
  - [Instalar Docker](https://docs.docker.com/get-docker/)
  - [Instalar docker-compose](https://docs.docker.com/compose/install/)

- **Python 3.11+** (opcional, si ejecutas sin Docker):
  ```bash
  pip install rdkit==2023.9.1
  ```

- **PostgreSQL 15+** o **SQLite 3.35+** (según backend elegido)

### Hardware Recomendado

- **RAM**: Mínimo 4GB, recomendado 8GB+ (para compilación y tests)
- **Disco**: 2GB libres para artifacts de compilación
- **CPU**: Soporte de múltiples cores (compilación paralela)

---

## 🚀 Instalación y Configuración

### 1. Clonar el repositorio

```bash
git clone https://github.com/chem-gl/chem-flow.git
cd flow-chem
```

### 2. Configurar variables de entorno

Crea un archivo `.env` en la raíz (opcional, los scripts usan defaults):

```bash
# Database
DATABASE_URL=postgres://admin:admin123@localhost:5432/mydatabase
CHEM_DB_URL=postgres://admin:admin123@localhost:5432/mydatabase

# Python/RDKit (si usas Docker, ya está configurado)
PYO3_PYTHON=/opt/conda/bin/python
PYTHON_SYS_EXECUTABLE=/opt/conda/bin/python

# SonarQube (opcional, solo para CI)
SONAR_TOKEN=your_token_here
```

### 3. Levantar servicios con Docker

```bash
# Inicia Postgres y contenedor de desarrollo
docker-compose up -d db app-dev

# Verifica que estén corriendo
docker-compose ps
```

---

## ▶️ Ejecución del Proyecto

### Con Docker (Recomendado)

El entorno Docker incluye:
- PostgreSQL 15 con healthcheck
- Contenedor `app-dev` con Rust, RDKit y Python 3.11

```bash
# Ejecutar shell dentro del contenedor de desarrollo
docker-compose exec app-dev bash

# Dentro del contenedor, compilar el proyecto
cargo build

# Ejecutar tests
cargo test --workspace

# Salir del contenedor
exit
```

### Localmente

Si tienes Rust, Python y RDKit instalados localmente:

```bash
# Compilar todo el workspace
cargo build --workspace

# Ejecutar tests con mocks (sin RDKit)
cargo test --workspace --features mock_rdkit

# Ejecutar tests con RDKit real (requiere Python + RDKit)
export PYO3_PYTHON=$(which python3)
cargo test --workspace
```

---

## 📝 Ejecución de Examples

El proyecto incluye un script interactivo para ejecutar ejemplos:

```bash
./scripts/run_examples.sh
```

**Menú interactivo:**
```
Selecciona qué ejemplo quieres ejecutar:
  1) example-domain (dominio)
  2) example-main (flow CLI)
  3) chem-persistence/persistence_simple_usage
  4) cadma_example (chem-workflow)
  5) Todos
```

### Ejemplos individuales

#### 1. Example Domain (CLI interactivo de dominio)

```bash
# Dentro de Docker
docker-compose exec app-dev bash
cargo run --example example-domain --features="integration_examples postgres pg_demo"

# Localmente con SQLite
export DATABASE_URL="sqlite:///tmp/flow_chem.db"
cargo run --example example-domain --features="integration_examples sqlite"
```

**Funcionalidades:**
- Crear moléculas desde partes o SMILES
- Crear y gestionar familias
- Agregar propiedades moleculares y familiares
- Listar y visualizar entidades

#### 2. Example Main (CLI interactivo de flows)

```bash
cargo run --example example-main --features="integration_examples postgres pg_demo"
```

**Funcionalidades:**
- Crear flows
- Agregar pasos (flow_data)
- Crear branches
- Guardar y cargar snapshots
- Visualizar estructura de árbol de flows

#### 3. CADMA Example (Workflow completo)

```bash
# Con Docker (recomendado, incluye RDKit)
./scripts/run_examples.sh  # Seleccionar opción 4

# Localmente
cargo run -p chem-workflow --example cadma_example --features="integration_examples pg postgres"
```

**Funcionalidades:**
- Ejecución interactiva de workflow CADMA
- Steps individuales (1-6)
- Creación de branches desde cualquier paso
- Integración completa: dominio + flows + RDKit

---

## 🧪 Testing

### Estrategia de Testing

El proyecto sigue **pirámide de tests**:

```
        /\
       /E2E\         ← Few (examples como tests de integración)
      /------\
     /  Integ \      ← Some (tests de repositorios y providers)
    /----------\
   /   Unit     \    ← Many (tests de dominio y lógica pura)
  /--------------\
```

### Comandos de Testing

#### Tests Unitarios (rápidos)

```bash
# Todos los tests unitarios con mocks
cargo test --workspace --lib --features mock_rdkit

# Tests de un crate específico
cargo test -p chem-domain
cargo test -p flow
```

#### Tests de Integración

```bash
# Tests de integración (requieren DB)
cargo test --workspace --test '*' --features testing

# Con Postgres (en Docker)
docker-compose exec app-dev cargo test --workspace --features postgres
```

#### Tests con RDKit Real

```bash
# Dentro de Docker (tiene RDKit instalado)
docker-compose exec app-dev bash
./scripts/run_tests_in_docker.sh

# O directamente
docker-compose exec app-dev cargo test --workspace
```

#### Tests End-to-End

```bash
# Los examples actúan como E2E tests
./scripts/run_examples.sh  # Opción 5 (Todos)
```

### Cobertura de Código

```bash
# Generar reporte de cobertura (LCOV y Cobertura XML)
./scripts/generate_coverage.sh

# Ver reporte HTML
open target/coverage/html/index.html  # macOS
xdg-open target/coverage/html/index.html  # Linux
```

**Targets de cobertura:**
- Dominio (`chem-domain`): >80%
- Flow (`flow`): >75%
- Persistence (`chem-persistence`): >70%
- Workflow (`chem-workflow`): >65%

---

## ✅ Calidad de Código

### Linting y Formateo

```bash
# Formatear todo el código (obligatorio antes de commit)
cargo fmt --all

# Verificar formato sin modificar
cargo fmt --all -- --check

# Linting con Clippy (estricto)
cargo clippy --workspace --all-targets -- -D warnings

# Auto-fix de Clippy (cuando sea posible)
cargo clippy --workspace --all-targets --fix
```

### Pre-commit Hook (Recomendado)

Crea `.git/hooks/pre-commit`:

```bash
#!/bin/bash
set -e

echo "🔍 Running pre-commit checks..."

# Format check
echo "  ➡️ Checking format..."
cargo fmt --all -- --check

# Linting
echo "  ➡️ Running clippy..."
cargo clippy --workspace --all-targets -- -D warnings

# Tests (rápidos con mocks)
echo "  ➡️ Running tests..."
cargo test --workspace --lib --features mock_rdkit

echo "✅ All checks passed!"
```

```bash
chmod +x .git/hooks/pre-commit
```

### SonarQube (CI/CD)

```bash
# Ejecutar análisis de SonarQube
./scripts/run_sonar.sh

# Requiere SONAR_TOKEN en variables de entorno
export SONAR_TOKEN=your_token
```

---

## 🌍 Variables de Entorno

| Variable | Descripción | Valor por Defecto | Requerido |
|----------|-------------|-------------------|-----------|
| `DATABASE_URL` | URL de conexión de base de datos para flows | `postgres://admin:admin123@db:5432/mydatabase` | Sí |
| `CHEM_DB_URL` | URL de conexión para dominio químico | Mismo que `DATABASE_URL` | Sí |
| `PYO3_PYTHON` | Ruta al ejecutable Python para PyO3 | `/opt/conda/bin/python` (Docker) | Si usas RDKit |
| `PYTHON_SYS_EXECUTABLE` | Alternativa a `PYO3_PYTHON` | `/opt/conda/bin/python` (Docker) | Si usas RDKit |
| `RUST_LOG` | Nivel de logging (`trace`, `debug`, `info`, `warn`, `error`) | `info` | No |
| `RUST_BACKTRACE` | Mostrar backtrace en panics (`0`, `1`, `full`) | `0` | No |
| `SONAR_TOKEN` | Token de autenticación SonarQube | - | Solo CI |

### Ejemplos de Configuración

#### Desarrollo Local con SQLite

```bash
export DATABASE_URL="sqlite:///tmp/flow_chem_dev.db"
export CHEM_DB_URL="sqlite:///tmp/flow_chem_dev.db"
```

#### Producción con PostgreSQL

```bash
export DATABASE_URL="postgres://user:pass@prod-host:5432/flowchem"
export CHEM_DB_URL="postgres://user:pass@prod-host:5432/flowchem"
```

#### Tests en Memoria

```bash
export DATABASE_URL="file:memdb1?mode=memory&cache=shared"
export CHEM_DB_URL="file:memdb1?mode=memory&cache=shared"
```

---

## 🔄 Flujos de Trabajo Principales

### Flujo 1: Crear y Persistir una Molécula

```mermaid
flowchart LR
    A[SMILES Input] --> B[ChemEngine::get_molecule]
    B --> C[Molecule::from_provider_molecule]
    C --> D[Validar InChIKey/SMILES]
    D --> E[DomainRepository::save_molecule]
    E --> F[(Database)]
    F --> G[Molécula Persistida]
```

**Código:**

```rust
use chem_providers::{ChemEngine, ChemEngineInterface};
use chem_domain::{Molecule, DomainRepository};
use chem_persistence::new_domain_from_env;

// 1. Inicializar engine y repositorio
let engine = ChemEngine::init()?;
let repo = new_domain_from_env()?;

// 2. Crear molécula desde SMILES
let smiles = "CCO";  // Etanol
let provider_mol = engine.get_molecule(smiles)?;
let molecule = Molecule::from_provider_molecule(smiles, provider_mol)?;

// 3. Persistir
let inchikey = repo.save_molecule(molecule)?;
println!("Molécula guardada: {}", inchikey);
```

---

### Flujo 2: Ejecutar Workflow CADMA Completo

```mermaid
flowchart TD
    Start[Inicio] --> CreateFlow[Crear Flow]
    CreateFlow --> Step1[Step1: Family Reference]
    Step1 --> Step2[Step2: ADMETSA Properties]
    Step2 --> Step3[Step3: Generate Initial Molecule]
    Step3 --> Step4[Step4: ADMETSA Initial]
    Step4 --> Step5[Step5: Generate Substitutions]
    Step5 --> Step6[Step6: ADMETSA Generated]
    Step6 --> Decision{Satisfactorio?}
    Decision -->|No| Branch[Crear Branch desde Step5]
    Branch --> Step5
    Decision -->|Sí| Snapshot[Guardar Snapshot]
    Snapshot --> End[Fin]
```

**Código simplificado:**

```rust
use chem_workflow::{ChemicalWorkflowFactory, CadmaFlow};

// 1. Crear workflow
let mut workflow = ChemicalWorkflowFactory::create::<CadmaFlow>("cadma-run-1")?;

// 2. Ejecutar steps secuencialmente
workflow.execute_step(0, &step1_input)?;  // Family
workflow.execute_step(1, &step2_input)?;  // ADMETSA
workflow.execute_step(2, &step3_input)?;  // Initial Molecule
workflow.execute_step(3, &step4_input)?;  // ADMETSA Initial
workflow.execute_step(4, &step5_input)?;  // Substitutions
workflow.execute_step(5, &step6_input)?;  // ADMETSA Generated

// 3. Guardar snapshot
workflow.save_snapshot()?;

// 4. Si queremos explorar alternativas, crear branch
let branch_id = workflow.create_branch_from_cursor(4)?;  // Desde Step5
```

---

### Flujo 3: Branching y Versionado

```mermaid
flowchart TD
    Main[Flow Principal] -->|Step 0| S0[Family]
    S0 -->|Step 1| S1[ADMETSA]
    S1 -->|Step 2| S2[Initial Mol]
    S2 -->|Step 3| S3[ADMETSA Init]
    S3 -->|Step 4| S4[Substitutions]
    
    S3 -.->|Branch A| BA0[Step 4: Alt Config]
    BA0 -->|Step 5| BA1[ADMETSA Gen A]
    
    S4 -->|Step 5| S5[ADMETSA Gen]
    S5 -.->|Branch B| BB0[Step 6: Refinement]
    
    style Main fill:#e3f2fd
    style BA0 fill:#fff3e0
    style BB0 fill:#f3e5f5
```

**Características:**
- Cada branch copia flow_data hasta el cursor especificado
- Branches son independientes: pueden divergir sin afectar al padre
- Snapshots permiten rehidratación rápida sin replay completo

---

## 🎓 Principios de Diseño

### SOLID

1. **Single Responsibility**: Cada crate/módulo tiene una responsabilidad clara
   - `chem-domain`: Lógica de negocio pura
   - `chem-persistence`: Solo persistencia
   - `chem-providers`: Solo integración con químicas

2. **Open/Closed**: Extensible sin modificar código existente
   - Nuevos steps de workflow sin cambiar engine
   - Nuevos providers sin cambiar dominio

3. **Liskov Substitution**: Implementaciones intercambiables
   - SQLite ↔ Postgres transparente
   - RDKit real ↔ Mock en tests

4. **Interface Segregation**: Traits pequeños y específicos
   - `MoleculeReader` / `MoleculeWriter` separados
   - `PropertyProvider` independiente de `ChemEngine`

5. **Dependency Inversion**: Depender de abstracciones
   - Dominio define ports (traits)
   - Infraestructura implementa adapters

### Domain-Driven Design (DDD)

- **Ubiquitous Language**: Términos del dominio químico en código
  - `Molecule`, `Family`, `ADMETSA`, `InChIKey`
- **Bounded Contexts**: Crates como contextos delimitados
  - Dominio químico vs. Gestión de flows
- **Aggregates**: `MoleculeFamily` como raíz de agregado
  - Controla acceso a `Molecule` y mantiene consistencia

### Clean Architecture

```
┌────────────────────────────────────┐
│     Frameworks & Drivers           │  ← chem-persistence, chem-providers
│  (Database, RDKit, External APIs)  │
└────────────────────────────────────┘
           ↑ Adapters ↑
┌────────────────────────────────────┐
│     Interface Adapters             │  ← Repositorios, Controllers
│      (Ports implementation)        │
└────────────────────────────────────┘
           ↑ Ports ↑
┌────────────────────────────────────┐
│      Application Business          │  ← chem-workflow (Use Cases)
│         Rules (Workflows)          │
└────────────────────────────────────┘
           ↑ Uses ↑
┌────────────────────────────────────┐
│      Enterprise Business           │  ← chem-domain, flow
│      Rules (Domain Entities)       │  (Entidades, Lógica Pura)
└────────────────────────────────────┘
```

### Inmutabilidad y Funcionalidad

- **Entidades inmutables**: Operaciones devuelven nuevas instancias
  ```rust
  let new_family = family.add_molecule(molecule)?;
  // family original no se modifica
  ```

- **Result para errores**: No panics en lógica de negocio
  ```rust
  fn save_molecule(&self, m: Molecule) -> Result<String, DomainError>
  ```

- **Builder pattern**: Construcción gradual de entidades
  ```rust
  let molecule = Molecule::from_parts(inchikey, smiles, inchi, metadata)?;
  ```

---

## 🤝 Contribución

### Workflow de Contribución

1. **Fork** el repositorio
2. **Crear branch** feature/fix desde `master`
   ```bash
   git checkout -b feature/nueva-funcionalidad
   ```
3. **Implementar** cambios con tests
4. **Verificar** calidad de código
   ```bash
   cargo fmt --all
   cargo clippy --workspace -- -D warnings
   cargo test --workspace --features mock_rdkit
   ```
5. **Commit** con mensajes descriptivos
   ```bash
   git commit -m "feat: agregar cálculo de LogP en Step2"
   ```
6. **Push** y crear **Pull Request**
   ```bash
   git push origin feature/nueva-funcionalidad
   ```

### Estilo de Commits (Conventional Commits)

- `feat:` Nueva funcionalidad
- `fix:` Corrección de bug
- `docs:` Cambios en documentación
- `refactor:` Refactorización sin cambios funcionales
- `test:` Agregar o mejorar tests
- `chore:` Tareas de mantenimiento (deps, configs)

**Ejemplo:**
```
feat(chem-workflow): add Step7 for toxicity prediction

- Implement ToxicityPredictionStep
- Add integration with external API
- Update workflow factory

Closes #42
```

### Code Review Checklist

- [ ] Tests pasan (`cargo test --workspace`)
- [ ] Sin warnings de Clippy
- [ ] Código formateado (`cargo fmt`)
- [ ] Documentación actualizada (README, rustdoc)
- [ ] Commits atómicos y descriptivos
- [ ] Sin secretos o credenciales en código

---

## 📄 Licencia

Este proyecto está licenciado bajo [MIT License](LICENSE).

---

## 📞 Contacto y Soporte

- **Issues**: [GitHub Issues](https://github.com/chem-gl/chem-flow/issues)
- **Discussions**: [GitHub Discussions](https://github.com/chem-gl/chem-flow/discussions)
- **Email**: maintainers@chem-flow.dev

---

## 🙏 Agradecimientos

- **RDKit Community**: Por la biblioteca química de código abierto
- **Rust Community**: Por el ecosistema de crates (Diesel, PyO3, Serde)
- **Contributors**: A todos los que han contribuido al proyecto

---

## 📚 Referencias y Recursos

### Documentación Técnica

- [Rust Book](https://doc.rust-lang.org/book/)
- [Diesel ORM Guide](https://diesel.rs/guides/)
- [PyO3 Documentation](https://pyo3.rs/)
- [RDKit Documentation](https://www.rdkit.org/docs/)

### Arquitectura y Patrones

- [Clean Architecture (Robert C. Martin)](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
- [Domain-Driven Design](https://martinfowler.com/bliki/DomainDrivenDesign.html)
- [Hexagonal Architecture](https://alistair.cockburn.us/hexagonal-architecture/)

### Papers y Referencias Químicas

- CADMA Methodology Papers
- ADMETSA Property Calculations
- InChI/InChIKey Standards

---

**Última actualización**: 12 de octubre de 2025

**Versión del documento**: 1.0.0

---

*Este README fue generado siguiendo los mejores estándares de ingeniería de software y documentación técnica. Para contribuir a su mejora, por favor abre un Pull Request.*
