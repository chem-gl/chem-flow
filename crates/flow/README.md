## Crate `flow` — documentación resumida en español

El crate `flow` define los tipos y traits que modelan la persistencia basada
en registros (`FlowData`) y provee implementaciones en memoria (stubs) y un
motor de ayuda (`FlowEngine`) para rehidratación y operaciones ergonomicas.

Contenido principal

- `FlowRepository` — trait que define el contrato de persistencia (Postgres,
  SQLite, in-memory, etc.).
- `InMemoryFlowRepository` — implementación en memoria para demos y pruebas.
- `FlowEngine` — helpers que usan el repositorio para operaciones comunes
  (crear flujo, añadir pasos, crear ramas, snapshots, etc.).
- Tipos de dominio: `FlowData`, `FlowMeta`, `SnapshotMeta`, `PersistResult`.

## Diagramas

### Diagrama de clases

```mermaid
classDiagram
  class FlowEngine {
    +new()
    +start_flow()
    +append()
    +get_items()
    +new_branch()
    +save_snapshot()
    +load_latest_snapshot()
  }
  class FlowRepository {
    <<trait>>
    +create_flow()
    +persist_data()
    +read_data()
    +create_branch()
    +save_snapshot()
    +load_snapshot()
  }
  class InMemoryFlowRepository
  class DieselFlowRepository
  class FlowData
  class FlowMeta
  class SnapshotMeta

  FlowEngine --> FlowRepository
  FlowRepository <|.. InMemoryFlowRepository
  FlowRepository <|.. DieselFlowRepository
  FlowEngine --> FlowData
  FlowEngine --> FlowMeta
  FlowEngine --> SnapshotMeta
```

### Diagrama de secuencia — creación de una rama (create_branch)

```mermaid
sequenceDiagram
  participant Caller
  participant Engine
  participant Repo
  participant DB

  Caller->>Engine: request new branch(parent_id, parent_cursor, metadata)
  Engine->>Repo: create_branch(parent_id, name, status, parent_cursor, metadata)
  Repo->>DB: SELECT flow_data WHERE flow_id = parent_id AND cursor <= parent_cursor
  Repo->>DB: SELECT snapshots WHERE flow_id = parent_id AND cursor <= parent_cursor
  Repo->>DB: INSERT new flow (parent reference, current_cursor = parent_cursor)
  Repo->>DB: INSERT copied flow_data rows for new flow
  Repo->>DB: INSERT copied snapshot rows for new flow
  DB-->>Repo: OK (ids)
  Repo-->>Engine: new_branch_id
  Engine-->>Caller: new_branch_id
```


Inicio rápido

1. Desde la raíz del workspace, ejecutar el ejemplo de uso en memoria:

```bash
cd crates/flow
cargo run --example flow_simple_usage
```

2. El ejemplo muestra: crear un flujo, añadir pasos, crear ramas y subramas,
   añadir pasos en las ramas, y consultar/depurar el contenido.

Principios y comportamiento

- Persistencia por registros: cada `FlowData` es autocontenido y permite
  ## Crate `flow` — estado actual y documentación (español)

  Este crate define los tipos y traits que modelan la persistencia basada en
  registros (`FlowData`) y proporciona un motor ligero (`FlowEngine`) más
  implementaciones en memoria para pruebas y demos. Complementariamente existe
  una implementación SQL en `crates/chem-persistence` (`DieselFlowRepository`) que
  provee persistencia durable usando Diesel + r2d2.

  Contenido principal

  - `FlowRepository` (trait): contrato de persistencia (crear flujos, persistir
    pasos, snapshots, crear ramas, listar, borrar, etc.).
  - `FlowEngine`: helpers que usan el repositorio para operaciones comunes
    (crear flujo, añadir pasos, crear ramas, snapshots, rehidratación).
  - Implementaciones:
    - `InMemoryFlowRepository` (stubs) — para pruebas y ejemplos.
    - `DieselFlowRepository` — implementación SQL en `crates/chem-persistence`
      (ver sección "Integración con chem-persistence").
  - Tipos de dominio: `FlowData`, `FlowMeta`, `SnapshotMeta`, `PersistResult`.

  Estado actual (qué está implementado)

  - Persistencia por registros: cada `FlowData` es autocontenido y permite
    reconstruir el estado mediante snapshot + replay.
  - Optimistic locking: `persist_data` en la implementación SQL usa
    `expected_version` y devuelve `PersistResult::Conflict` si hay desajuste.
  - `create_flow`: inserta una fila en `flows` con `current_cursor = 0` y
    `current_version = 0` y devuelve el `Uuid` generado.
  - `persist_data`: inserta la fila en `flow_data` y actualiza `flows.current_cursor`
    y `flows.current_version` de forma atómica (transactional en la impl. SQL).
  - `read_data(flow_id, from_cursor)`: devuelve `FlowData` con `cursor > from_cursor`,
    ordenado ascendentemente.
  - Snapshots: `save_snapshot`, `load_snapshot`, `load_latest_snapshot` están
    implementados en la capa SQL; actualmente el contenido del snapshot se
    guarda en la columna `state_ptr` como texto (no hay object-store aún).
  - Branching: `create_branch(parent_id, parent_cursor, ...)` crea una nueva
    fila en `flows` y copia (en la base de datos) todas las filas de `flow_data`
    y `snapshots` del padre cuyo `cursor <= parent_cursor`. La operación es
    transaccional en la implementación SQL.
  - `delete_branch`: elimina `flow_data` y `snapshots` del branch y orfana a los
    hijos (pone `parent_flow_id` y `parent_cursor` a NULL); no borra recursivamente
    ramas hijas.
  - Operaciones no implementadas / esqueleto: `delete_from_step` devuelve
    `not implemented`; `claim_work`, `SnapshotStore` (persistencia externa de
    blobs) y `ArtifactStore` están esbozados y devuelven errores por ahora.

  Integración con `crates/chem-persistence`

  - `DieselFlowRepository` vive en `crates/chem-persistence` y expone una
    implementación SQL del trait `FlowRepository`:
    - Usa Diesel + r2d2.
    - Por defecto los tests usan SQLite en memoria; la feature `pg` permite
      compilar y usar Postgres en producción.
    - Aplica migraciones embebidas al arrancar (migrations en
      `crates/chem-persistence/migrations`).
    - Tablas principales (ver `schema.rs`): `flows`, `flow_data`, `snapshots`.
    - Además existen tablas relacionadas con el dominio químico: `molecules`,
      `families`, `family_properties`, `molecular_properties`, `family_members`.

  Esquema (resumen)

  - Tabla `flows` (id TEXT PK): metadata del flujo, `current_cursor`,
    `current_version`, `parent_flow_id`, `parent_cursor`, `metadata` (JSON).
  - Tabla `flow_data` (id TEXT PK): `flow_id`, `cursor`, `key`, `payload` (JSON),
    `metadata` (JSON), `command_id`, `created_at_ts`.
  - Tabla `snapshots` (id TEXT PK): `flow_id`, `cursor`, `state_ptr` (texto/URI),
    `metadata` (JSON), `created_at_ts`.

  Migraciones

  - Las migraciones SQL están en `crates/chem-persistence/migrations/`. Hay al
    menos dos migraciones: la creación del esquema básico (`flows`,
    `flow_data`, `snapshots`) y las tablas químicas (`molecules`, `families`,
    `family_properties`, `molecular_properties`, `family_members`).

  Cómo ejecutar (rápido)

  - Ejecutar tests del crate `flow`:

  ```bash
  cargo test -p flow
  ```

  - Ejecutar el ejemplo local de `flow` (InMemory):

  ```bash
  cd crates/flow
  cargo run --example flow_simple_usage
  ```

  - Para la implementación SQL (chem-persistence):

  ```bash
  # Tests (usa sqlite in-memory por defecto)
  cargo test -p chem-persistence

  # Ejemplo de persistencia:
  cargo run -p chem-persistence --example persistence_simple_usage
  ```

  - Para usar Postgres (producción/demo con DB real), exporta `DATABASE_URL`
    y compila con la feature `pg` cuando proceda. Por ejemplo:

  ```bash
  export DATABASE_URL=postgres://user:pass@localhost:5432/dbname
  cargo run -p chem-persistence --features pg --example persistence_simple_usage
  ```

  Limitaciones conocidas y siguientes pasos

  - Snapshot storage: actualmente los snapshots se guardan como texto en
    `state_ptr`. Se recomienda implementar un `SnapshotStore` que guarde blobs
    en un object store (S3/MinIO) y deje `state_ptr` como key/URI.
  - `ArtifactStore` y métodos relacionados aún no están implementados en la
    versión SQL (devuelven errores `not implemented`).
  - `delete_branch` orfana hijos en lugar de borrado recursivo. Si necesitas
    borrado en cascada implementa lógica adicional y tests.
  - Se recomienda añadir pruebas de integración que validen explícitamente la
    copia de `snapshots` al crear ramas y pruebas de concurrencia para
    `persist_data`.

  Contribuciones

  Si deseas ayudar a mejorar el crate `flow` o la implementación SQL:

  - Implementa `SnapshotStore` con object store.
  - Completa `ArtifactStore` con una implementación estable.
  - Añade pruebas de integración para Postgres (activar `pg` feature en CI)
    y escenarios de concurrencia.

  Contacto

  Revisar el código fuente en `crates/flow/src/` y la implementación SQL en
  `crates/chem-persistence/` para detalles de las funciones. Para cambios más
  grandes abre un issue o PR en el repositorio.
