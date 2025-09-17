# Crate `flow` — Ejemplo y documentación

Este crate provee una capa de persistencia orientada a datos para flujos
de trabajo. Su responsabilidad es almacenar `FlowData`, snapshots y
crear ramas; NO ejecuta la lógica de negocio de los pasos.

Resumen rápido:

- `FlowRepository` — trait que define el contrato para persistencia.
- `InMemoryFlowRepository` — implementación en memoria útil para demos.
- `FlowEngine` — helpers ergonomicos: `append_flow_data`, `read_data`,
  `save_snapshot`, `create_branch`, `persist_data`.
- `FlowService` — capa de orquestación pensada para invocarse desde
  handlers HTTP o workers.

Ejemplo rápido (ver `examples/simple_usage.rs`):

- Crear repo en memoria
- Crear flow
- Persistir un `FlowData` con `append_flow_data`
- Leer los `FlowData`

Cómo ejecutar el ejemplo:

```bash
# desde la raíz del workspace
cd crates/flow
cargo run --example simple_usage
```

Notas:

- Este crate está pensado como base para una implementación Postgres + S3
  en producción; `InMemoryFlowRepository` es solo para desarrollo y demos.
- Respeta idempotencia mediante `command_id` y locking optimista mediante
  `expected_version`.
