
# Crate `chem-persistence` (documentación en español)

`chem-persistence` implementa una persistencia mínima para el trait
`FlowRepository` definido en el crate `flow`. Usa Diesel como capa de acceso
a base de datos y ofrece migraciones embebidas para inicializar el esquema.
Está pensado como referencia y como backend ligero para pruebas y demos.

Resumen rápido
- Soporta SQLite (por defecto para tests) y Postgres (activando la feature
  `pg`).
- Mantiene tres tablas principales: `flows`, `flow_data` y `snapshots`.
- Provee operaciones atómicas para persistir pasos (`FlowData`), crear ramas
  (branching) copiando pasos y snapshots hasta un cursor, y eliminar ramas.

## Objetivos y responsabilidades

- Persistir metadatos de flujos (`flows`) y registros autocontenidos
  (`flow_data`) que permiten reconstruir el estado mediante replay.
- Guardar metadata de snapshots en la tabla `snapshots` (el contenido del
  estado puede almacenarse en `state_ptr` como texto o como referencia a un
  object store, dependiendo de la implementación futura del `SnapshotStore`).
- Ofrecer helpers para inicializar la base de datos (migraciones embebidas)
  y funciones utilitarias para debugging y ejemplos.

## Estructuras y API pública (resumen)

Nota: aquí se documenta el comportamiento observable de los métodos públicos.
Para la definición del trait y las firmas exactas consulta `crates/flow/src/repository.rs`.

- `DieselFlowRepository`
  - Implementación de `FlowRepository` usando Diesel + r2d2.
  - Métodos de interés:
    - `new(database_url: &str) -> Result<DieselFlowRepository, FlowError>`: crea un repo
      con la URL indicada (SQLite o Postgres según la URL y features).
    - `new_from_env() -> Result<DieselFlowRepository, FlowError>`: lee `DATABASE_URL`
      (usa `dotenvy` si existe `.env`), crea el pool y aplica migraciones
      embebidas antes de devolver el repo.
    - `dump_tables_for_debug() -> Result<(Vec<FlowRow>, Vec<FlowDataRow>), FlowError>`:
      helper para tests y depuración que devuelve las filas actuales de
      `flows` y `flow_data`.

- Comportamiento clave del `FlowRepository` implementado:
  - `create_flow(name, status, metadata)`: inserta una fila en `flows` con
    `current_cursor=0` y `current_version=0` y devuelve el `Uuid` generado.
  - `persist_data(fd, expected_version)`: inserta un registro en `flow_data`
    y actualiza `flows.current_cursor`/`current_version` de forma atómica.
    Usa locking optimista: si `expected_version` no coincide devuelve
    `PersistResult::Conflict`.
  - `read_data(flow_id, from_cursor)`: devuelve los `FlowData` con `cursor > from_cursor`.
  - `create_branch(parent_flow_id, name, status, parent_cursor, metadata)`: crea
    una nueva fila en `flows` con referencia al padre y copia (en la BD) todos
    los `flow_data` y snapshots del padre con `cursor <= parent_cursor`.
    La operación es transaccional.
  - `branch_exists(flow_id)`, `count_steps(flow_id)` y `delete_branch(flow_id)`:
    utilidades habituales; `delete_branch` elimina datos y snapshots del
    branch y, en la implementación actual, "orfana" a los hijos (los hijos
    mantienen `parent_flow_id = NULL` en lugar de borrarse recursivamente).
  - `save_snapshot`, `load_snapshot`, `load_latest_snapshot`: gestión básica
    de snapshots; actualmente `state_ptr` se guarda como texto y se devuelve
    como bytes por `load_snapshot`.

## Limitaciones y decisiones de diseño

- Este crate es una implementación de referencia: orientado a pruebas y demos.
- Por defecto `diesel` está configurado para SQLite en los tests. Para usar
  Postgres activa la feature `pg` en `crates/chem-persistence/Cargo.toml`.
- `delete_branch` orfana hijos en vez de eliminarlos recursivamente. Esto
  evita borrados accidentales de subárboles; adapta la lógica si necesitas
  otra semántica (por ejemplo, borrado recursivo).
- Los stores de artifacts/snapshots están esbozados aquí; para producción se
  recomienda un `SnapshotStore` que guarde estados en un object store (S3,
  MinIO) y deje `state_ptr` como key/URI.

## Migraciones

Las migraciones SQL están en `migrations/00000000000001_create_schema/`.
`new_from_env()` aplica las migraciones embebidas automáticamente al crear
el repositorio (cuando procede), por lo que normalmente no necesitas ejecutar
comandos manuales de migración en entornos de prueba o desarrollo.

## Cómo ejecutar y probar

Desde la raíz del workspace puedes ejecutar los tests y ejemplos locales:

```bash
# ejecutar tests (usa sqlite en memoria por defecto)
cargo test -p chem-persistence

# ejecutar el ejemplo de persistencia (usa DATABASE_URL si está definido)
cargo run -p chem-persistence --example persistence_simple_usage
```

Notas rápidas:

- Para pruebas locales se usa SQLite en memoria salvo que se compile con la
  feature `pg`.
- Para producción activa la feature `pg` y proporciona una `DATABASE_URL`
  apuntando a Postgres.

## Contribuciones y mantenimiento

Si vas a extender este crate para producción, considera implementar un
`SnapshotStore` que guarde blobs en un object store (S3/MinIO) y un
`ArtifactStore` estable para los artefactos pesados. Añade también pruebas
de integración apuntando a una BD Postgres dedicada cuando actives `pg`.
el repositorio, por lo que normalmente no necesitas ejecutar `diesel migration`.

## Cómo probar y ejecutar (comandos)

Desde la raíz del workspace:

- Ejecutar tests del crate `chem-persistence` (SQLite in-memory):

```bash
cargo test -p chem-persistence
```

- Ejecutar el ejemplo que usa DB en memoria (demo):

```bash
cd crates/chem-persistence
cargo run --example simple_usage
```

- Ejecutar la aplicación principal `main-core` usando Postgres (asegúrate
  de exportar `DATABASE_URL` o crear `.env`):

```bash
export DATABASE_URL=postgres://admin:admin123@localhost:5432/mydatabase
cargo run -p main-core --features pg_demo
```

- Si usas Docker Compose (archivo `docker-compose.yml` incluido), el
  servicio de BD se llama `db` y puedes usar:

```bash
# from repo root
docker compose up -d db
# luego dentro del contenedor o desde el host (host=localhost vs db según contexto):
export DATABASE_URL=postgres://admin:admin123@db:5432/mydatabase
cargo run -p main-core --features pg_demo
```

## Buenas prácticas para QA

- Para pruebas rápidas usa la URL SQLite en memoria:
  `file:memdb_<uuid>?mode=memory&cache=shared`.
- Verifica branching creando un flujo, añadiendo pasos, creando una rama
  con `parent_cursor` y comprobando que los pasos hasta ese cursor se copiaron
  en la nueva rama.

## Desarrollo y puntos futuros

- Implementar `SnapshotStore` sobre un object store (S3/MinIO) y usar claves
  en `state_ptr` en lugar de texto serializado.
- Optimizar la copia de artifacts al crear ramas (copy-on-write o referencias).
- Añadir tests que validen explícitamente la copia de snapshots durante
  `create_branch`.

Si quieres, puedo además:

- Añadir tests automáticos que verifiquen la copia de snapshots al crear ramas.
- Actualizar `README.md` del crate `flow` para unificar ejemplos y firmas.

Fin del README.
```
