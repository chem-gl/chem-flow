# Flow-Chem — descripción del workspace (en español)

Este repositorio contiene una pequeña plataforma de demostración para
persistencia de flujos basada en registros (`FlowData`) y ejemplos de
implementaciones (in-memory y Diesel). Está pensado como proyecto de
referencia, pruebas y ejemplos educativos.

Estructura principal

- `crates/flow`: definiciones de dominio (`FlowData`, `FlowMeta`), el trait
  `FlowRepository`, implementación en memoria (`InMemoryFlowRepository`) y
  `FlowEngine` con helpers para crear flujos, ramas y gestionar snapshots.

- `crates/chem-persistence`: implementación basada en Diesel (SQLite/Postgres)
  del trait `FlowRepository`.

- `crates/chem-domain`: tipos del dominio químico (moléculas, propiedades).

- `src/main.rs`: binario principal (`main-core`).

- `examples/`: ejemplos ejecutables (por ejemplo `example-main`).

Cómo ejecutar

- Ejecutar el binario principal (por defecto):

```bash
cargo run --bin main-core
```

- Ejecutar el ejemplo CLI movido a `examples/`:

```bash
cargo run --example example-main
```

- Ejecutar ejemplos del crate `flow`:

```bash
cd crates/flow
cargo run --example flow_simple_usage
```

Notas sobre configuración

- Muchas operaciones dependen de `DATABASE_URL` cuando se usa
  `chem-persistence` (Postgres o SQLite). Para desarrollo puedes usar
  SQLite en memoria: `file:memdb_...` o exportar una URL a Postgres.
- Revisa el archivo `.env.example` en la raíz para variables recomendadas.

Documentación y comentarios

- Los crates incluyen comentarios y documentación en español en archivos
  clave (`crates/flow/src/lib.rs`, `crates/flow/src/repository.rs`, etc.).
  Si deseas generar documentación con `cargo doc`, ejecuta:

```bash
cargo doc --workspace --no-deps --open
```

Contribuir

- Para añadir ejemplos o mejorar la persistencia, crea ramas y abre pull
  requests con pruebas que validen el comportamiento (especialmente
  operaciones de branching y borrado).
