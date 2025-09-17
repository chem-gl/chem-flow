# Crate `chem-persistence`

Este crate proporciona una implementación mínima y documentada de persistencia
para el trait `FlowRepository` del workspace `flow`, usando Diesel y migraciones
embebidas (`diesel_migrations`). Está pensada como referencia y base para pruebas
(y para demos con SQLite). También contiene soporte opcional para Postgres
cuando se activa la feature `pg`.

> Documentación en español: visión general, diagramas, uso, y referencia de cada
> archivo y función pública.

## Objetivo

- Implementar un repositorio transaccional simple que guarde:
  - Metadatos de flows en la tabla `flows`.
  - Registros autocontenidos (`FlowData`) en la tabla `flow_data`.
  - Snapshots en la tabla `snapshots` (metadatos + pointer a estado serializado).
- Proveer utilidades para aplicar migraciones embebidas (al inicializar el
  repositorio mediante `new_from_env`).
- Soportar la creación de ramas (branches) copiando los pasos hasta un cursor
  dado y, además, copiar snapshots relevantes.
- Ofrecer tests y ejemplos que demuestren persistencia, branching y borrado de
  ramas.

## Diagrama (clase simplificado)

```mermaid
classDiagram
    class FlowRepository {
        +create_flow(name, status, metadata) FlowResult<Uuid>
        +persist_data(fd, expected_version) FlowResult<PersistResult>
        +read_data(flow_id, from_cursor) FlowResult<Vec<FlowData>>
        +create_branch(parent_id, name, status, parent_cursor, metadata) FlowResult<Uuid>
        +branch_exists(flow_id) FlowResult<bool>
        +count_steps(flow_id) FlowResult<i64>
        +delete_branch(flow_id) FlowResult<()>
        +save_snapshot(flow_id, cursor, state, metadata) FlowResult<Uuid>
        +load_snapshot(snapshot_id) FlowResult<(Vec<u8>, SnapshotMeta)>
        +load_latest_snapshot(flow_id) FlowResult<Option<SnapshotMeta>>
    }

    class DieselFlowRepository {
        -pool: Arc<DbPool>
        +new(database_url) -> Self
        +new_from_env() -> Self
        +dump_tables_for_debug() -> (Vec<FlowRow>, Vec<FlowDataRow>)
    }

    FlowRepository <|.. DieselFlowRepository
```

## Diagrama de secuencia (crear rama)

```mermaid
sequenceDiagram
    participant Caller
    participant Repo as DieselFlowRepository
    Caller->>Repo: create_branch(parent_id, name, .., parent_cursor)
    Repo->>Repo: BEGIN TRANSACTION
    Repo->>Repo: SELECT flow_data WHERE flow_id=parent_id AND cursor <= parent_cursor
    Repo->>Repo: INSERT new flow row (parent_flow_id=parent_id, parent_cursor=parent_cursor)
    Repo->>Repo: INSERT copies of selected flow_data rows with new flow_id
    Repo->>Repo: INSERT copies of snapshots with cursor <= parent_cursor for new flow
    Repo->>Repo: COMMIT
    Repo-->>Caller: new_branch_id
```

## Contenido y archivos

Lista de archivos en el crate y su propósito:

- `Cargo.toml` — Dependencias y features. Importante:

  - Feature `pg` habilita soporte a Postgres en Diesel.
  - `diesel` está configurado para `sqlite` por defecto (útil para tests rápidos).

- `migrations/00000000000001_create_schema/up.sql` — crea las tablas `flows`,
  `flow_data` y `snapshots`.

- `migrations/00000000000001_create_schema/down.sql` — elimina las tablas (rollback).

- `src/lib.rs` — Implementación principal.

  - Expone `DieselFlowRepository` y la función `new_from_env()`.
  - Implementa el trait `FlowRepository` requerido por el crate `flow`.
  - Implementa (parcialmente) `SnapshotStore` y `ArtifactStore` con stubs que
    retornan errores si se usan (no están implementadas aquí).
  - Define las structs internas `FlowRow`, `FlowDataRow`, `SnapshotRow` que
    mapean las tablas de Diesel.
  - Implementa utilidades:
    - `new_from_env()` — carga `.env` (si existe) y construye el pool de
      conexiones, además aplica migraciones embebidas.
    - `dump_tables_for_debug()` — devuelve los rows de `flows` y `flow_data`
      para inspección durante demos/tests.

- `src/schema.rs` — macros de Diesel que describen las tablas `flows`,
  `flow_data`, `snapshots`.

- `tests/integration_tests.rs` — pruebas de integración que usan SQLite
  en memoria (`file:memdb1?mode=memory&cache=shared`) y verifican:

  - Crear flow.
  - Persistir varios pasos.
  - Crear branch en un cursor dado y verificar que los pasos se copiaron.
  - Crear y eliminar una rama temporal.

- `examples/simple_usage.rs` — ejemplo que fuerza la DB in-memory y demuestra
  la API del repo: crear flows, añadir pasos, crear ramas, borrar ramas y
  volcar tablas.

## Cómo usar

Requisitos:

- Rust toolchain (cargo).
- Para usar SQLite (tests y ejemplos), no se necesita nada adicional.
- Para usar Postgres en `main-core` o en producción:
  - Exporta `DATABASE_URL` apuntando a un Postgres accesible.
  - Habilita la feature `pg` si quieres compilar con Diesel/Postgres:
    `--features pg` o activar la feature de workspace `pg_demo` si se ha
    configurado.
  - El crate ejecuta migraciones embebidas al inicializar el repositorio
    mediante `new_from_env()`.

Ejemplos rápidos:

- Ejecutar las pruebas de integración (SQLite in-memory):

```bash
# Desde la raíz del workspace
cargo test -p chem-persistence
```

- Ejecutar el ejemplo que usa DB en memoria:

```bash
cd crates/chem-persistence
cargo run --example simple_usage
```

- Ejecutar el binario principal (`main-core`) usando el repo y aplicando
  migraciones desde `.env` (Postgres):

```bash
# Establece DATABASE_URL en tu entorno o en .env
export DATABASE_URL=postgres://admin:admin123@localhost:5432/mydatabase
# Desde la raíz del workspace (si se habilitó la feature):
cargo run -p main-core --features pg_demo
```

> Nota: si corres dentro de Docker Compose (servicio `app-dev`), usa host
> `db` para `DATABASE_URL` (ej. `postgres://admin:admin123@db:5432/mydatabase`).

## API y documentación de funciones públicas

A continuación se documentan las funciones y métodos públicos expuestos por
este crate (resumen y notas de comportamiento). Los nombres y firmas pueden
variar ligeramente según la versión en el workspace; la idea es dar un mapa
completo de responsabilidades.

### Estructuras públicas

- `DieselFlowRepository` — implementación de `FlowRepository` usando Diesel.

  - Campos:

    - `pool: Arc<DbPool>` — pool de conexiones (r2d2) a la base de datos.

  - Métodos públicos principales:

    - `new(database_url: &str) -> FlowResult<DieselFlowRepository>`

      - Crea un `DieselFlowRepository` apuntando a `database_url` (SQLite o
        Postgres según la URL y features).
      - No ejecuta migraciones automáticamente; si quieres migraciones,
        usa `new_from_env()` o llama manualmente al harness.
      - Errores: retorna `FlowError::Storage` si no puede conectar.

    - `new_from_env() -> FlowResult<DieselFlowRepository>`

      - Lee `DATABASE_URL` del entorno (usa `dotenvy::dotenv()` para cargar
        `.env` si existe) y construye el repositorio.
      - Ejecuta migraciones embebidas (las migraciones en `migrations/`.
      - Uso típico en `main`: garantiza que las tablas existen antes de usar
        el repo.

    - `dump_tables_for_debug() -> FlowResult<(Vec<FlowRow>, Vec<FlowDataRow>)>`
      - Helper para tests y debugging: devuelve los rows de `flows` y `flow_data`.
      - No es eficiente ni pensada para producción.

### Implementación de `FlowRepository` (resumen de comportamiento)

Estas son las responsabilidades que cubre la implementación (métodos del
trait `FlowRepository`). Para cada método se resume el comportamiento y
los errores esperados.

- `create_flow(name: Option<String>, status: Option<String>, metadata: JsonValue) -> FlowResult<Uuid>`

  - Inserta una fila en `flows` con `current_cursor=0` y `current_version=0`.
  - Devuelve el `Uuid` generado para el flow.

- `get_flow_meta(flow_id: &Uuid) -> FlowResult<FlowMeta>`

  - Lee los metadatos del flow desde `flows`.
  - `FlowError::NotFound` si el flow no existe.

- `persist_data(fd: &FlowData, expected_version: i64) -> FlowResult<PersistResult>`

  - Persiste un `FlowData` en `flow_data` y actualiza `flows.current_cursor`
    y `flows.current_version` de forma atómica.
  - Implementa locking optimista: el caller debe proporcionar
    `expected_version`; si no coincide con la versión actual se retorna
    `PersistResult::Conflict`.
  - Devuelve `PersistResult::Ok { new_version }` con la nueva versión.
  - Internamente usa una transacción para asegurar atomicidad.

- `read_data(flow_id: &Uuid, from_cursor: i64) -> FlowResult<Vec<FlowData>>`

  - Devuelve los `FlowData` con `cursor > from_cursor` ordenados por cursor.

- `create_branch(parent_flow_id: &Uuid, name: Option<String>, status: Option<String>, parent_cursor: i64, metadata: JsonValue) -> FlowResult<Uuid>`

  - Crea una nueva fila en `flows` con `parent_flow_id` y `parent_cursor`.
  - Copia (INSERT) todos los `flow_data` del padre con `cursor <= parent_cursor`
    usando la nueva `flow_id`.
  - Copia (INSERT) todos los snapshots del padre con `cursor <= parent_cursor`
    y los asocia a la nueva `flow_id`.
  - Establece `current_cursor` y `current_version` del nuevo flow acorde al
    número de pasos copiados.
  - Realiza la operación en una transacción para mantener la consistencia.

- `branch_exists(flow_id: &Uuid) -> FlowResult<bool>`

  - Verifica si existe una fila `flows` con ese `id`.

- `count_steps(flow_id: &Uuid) -> FlowResult<i64>`

  - Cuenta los pasos (`flow_data`) asociados al `flow_id`.

- `delete_branch(flow_id: &Uuid) -> FlowResult<()>`

  - Elimina en transacción las filas `flow_data` y la fila `flows` para el
    `flow_id` dado. No toca snapshots en el diseño actual (podría hacerlo si
    fuera necesario).

- `save_snapshot(flow_id: &Uuid, cursor: i64, state_bytes: &str, metadata: JsonValue) -> FlowResult<Uuid>`

  - Inserta una fila en `snapshots` con `state_ptr` apuntando a una cadena
    que representa el estado (en este crate `state_ptr` puede ser simplemente
    el JSON serializado o un marcador a un blob externo en un object store).
  - Retorna el `Uuid` de la snapshot insertada.

- `load_snapshot(snapshot_id: &Uuid) -> FlowResult<(Vec<u8>, SnapshotMeta)>`

  - Devuelve los bytes del estado (por ahora el crate guarda el JSON como
    texto en `state_ptr`, por lo que devuelve `state_ptr` como bytes) y la
    metadata de snapshot.

- `load_latest_snapshot(flow_id: &Uuid) -> FlowResult<Option<SnapshotMeta>>`
  - Devuelve la snapshot con mayor `cursor` para el `flow_id`, o `None` si
    no hay snapshots.

## Limitaciones y notas de diseño

- Esta implementación está orientada a pruebas y demos: usa SQLite por defecto
  (configurado en `diesel` features) y los stores de artifacts/snapshot están
  esbozados, no son un object store real (S3, MinIO).

- Para producción se recomienda una implementación sobre Postgres (activar
  feature `pg`) y separar el almacenamiento grande de snapshots/artifacts en
  un object store. `create_branch` debe adaptarse si el store de artifacts usa
  copy-on-write o referencias (actualmente copia metadatos de snapshots).

- Concurrencia y locking: el crate usa un esquema de locking optimista con
  `current_version`. Para cargas altas o multi-nodo puede necesitarse
  mecanismos adicionales (advisory locks, filas de trabajo, o cola externa).

- `dump_tables_for_debug()` es un helper para inspección en demos/tests y no
  debe usarse como API pública en un entorno de producción por razones de
  rendimiento y seguridad.

## Ejemplos de verificación (recomendado para QA)

1. Ejecutar `cargo test -p chem-persistence` para validar integraciones que
   usan SQLite in-memory.
2. Ejecutar `cargo run --example simple_usage` para ver un flujo completo en la
   consola.
3. Levantar Postgres con Docker Compose y ejecutar `cargo run -p main-core`
   (con `DATABASE_URL` apuntando al servicio `db`) para verificar migraciones
   aplicadas y datos persistidos.

## Desarrollo y extensiones futuras

- Implementar `SnapshotStore` sobre un object store real y guardar `state_ptr`
  como key hacia ese store (S3/MinIO).
- Añadir soporte de copia eficiente de artifacts al crear ramas (copy-on-write).
- Mejorar los tests para cubrir la copia de snapshots durante `create_branch` y
  asegurar que una snapshot con cursor <= parent_cursor está disponible en la
  rama recién creada.

---

Si quieres, puedo:

- Añadir tests adicionales que verifiquen explícitamente que las snapshots
  con cursor <= parent_cursor son copiadas a la rama.
- Actualizar el README con comandos específicos para ejecutar dentro del
  contenedor `app-dev` (docker-compose) si quieres la guía paso-a-paso.

Fin del README.
