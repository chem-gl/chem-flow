# Diseño: modelo por FlowData

Reescrito para el modelo por FlowData — diseño compacto y coherente (comentarios en español).

Este documento describe la arquitectura y contratos del crate `flow`.
El foco está en persistir registros de datos autocontenidos (`FlowData`),
crear ramas, snapshots y facilitar la rehidratación por un motor externo.

## Diagrama unificado de componentes / clases

```mermaid
classDiagram
    direction TB

    class FlowService {
      +startFlow(templateId, params) -> FlowId
      +createBranchFromSnapshot(flowId, snapshotId|cursor, reason) -> FlowId
      +getFlowStatus(flowId) -> FlowStatus
      # Diseño: modelo por FlowData

      Reescrito para el modelo por FlowData — diseño compacto y coherente (comentarios en español).

      Este documento describe la arquitectura y contratos del crate `flow`.
      El foco está en persistir registros de datos autocontenidos (`FlowData`),
      crear ramas, snapshots y facilitar la rehidratación por un motor externo.

      ## Diagrama unificado de componentes / clases

      ```mermaid
      classDiagram
          direction TB

          class FlowService {
            +start_flow(name, status, metadata) -> FlowId
            +create_branch_from_snapshot(parent_flow_id, parent_cursor, name, status, metadata) -> FlowId
            +get_flow_status(flow_id) -> Option<String>
            +claim_work(worker_id) -> WorkItem | null
          }

          class FlowEngine {
            +new(repo: FlowRepository, engineConfig)
            +rehydrate(snapshotState?, steps[])
            +append_flow_data(flow_id, key, payload, metadata, command_id, expected_version)
            +read_data(flow_id, from_cursor)
            +save_snapshot(flow_id, cursor, state_ptr, metadata)
            +create_branch(parent_flow_id, name, status, parent_cursor, metadata)
          }

          class FlowRepository {
            <<interface>>
            +get_flow_meta(flowId) -> FlowMeta
            +create_flow(name, status, metadata) -> newFlowId
            +read_data(flowId, from_cursor) -> List<FlowData>
            +persist_data(flowId, data, expected_version) -> PersistResult
            +save_snapshot(flow_id, cursor, state_ptr, metadata) -> snapshotId
            +create_branch(parent_flow_id, parent_cursor, name, status, metadata) -> newFlowId
            +lock_for_update(flowId, expected_version) -> bool
            +claim_work(workerId) -> WorkItem | null
          }

          class SnapshotStore {
            +save(snapshotBytes) -> key
            +load(key) -> bytes
          }

          FlowService --> FlowRepository
          FlowService --> FlowEngine
          FlowEngine --> FlowRepository
          FlowRepository --> SnapshotStore
      ```

      ---

      # Máquina de estados por FlowData / registros de datos (compacta y explícita)

      ```mermaid
      stateDiagram-v2
          [*] --> Pending
          Pending --> Running: StepStarted
          Running --> Completed: StepCompleted
          Running --> Failed: StepFailed
          Pending --> Skipped: StepSkipped
          Failed --> Pending: RetryScheduled / ManualRetry
          Pending --> Cancelled: StepCancelled
          Completed --> [*]
          Skipped --> [*]
          Cancelled --> [*]
      ```

      ---

      # Arquitectura de almacenamiento (Postgres + Object Store)

      Este documento describe el contrato del crate `flow`. El crate es
      una capa de persistencia y gestión de datos: guarda `FlowData`, crea
      ramas (branches), snapshots y devuelve la información necesaria para que
      un motor externo rehidrate y ejecute la lógica de negocio.

      Principios clave:

      - El `FlowEngine` de este crate NO ejecuta pasos ni workflows. Sólo
        expone helpers para persistir/leer datos, crear ramas y guardar
        snapshots.
      - Persistencia append-only para `FlowData` (tabla `steps`).
      - Idempotencia mediante `command_id` y bloqueo optimista mediante
        `expected_version`.

      ## Componentes mínimos

      - `FlowRepository` — contrato para persistencia (Postgres + ObjectStore).
      - `FlowEngine` — helpers para persistir/leer/snapshots (no-ejecutor). Las ramas se crean manualmente mediante `FlowService::create_branch_from_snapshot`.
      - `FlowService` — capa de orquestación para API/operaciones atómicas.
      - `InMemoryFlowRepository` — implementación de desarrollo.

      ## Secuencia simplificada: crear una rama desde snapshot/cursor

      1. El caller (API/handler) invoca `FlowService::create_branch_from_snapshot`
         pasando `parent_flow_id`, `parent_cursor`, `name`, `status` y `metadata`.
      2. `FlowService` delega en `FlowRepository.create_branch`, que debe realizar
         de forma atómica: crear la fila en `flows`, copiar los `FlowData` del
         padre hasta `parent_cursor` (si aplica) y añadir un `FlowData` tipo
         `BranchCreated` en `steps` para la nueva rama.
      3. `create_branch` devuelve el nuevo `flow_id` (UUID) del branch creado.

      Nota: por defecto no se copian blobs (`state_ptr`). Copiar solo si la
      rama necesita aislamiento (copy-on-write).

      ## Snapshot: guardar estado

      1. Serializar estado y subir a ObjectStore → obtener `state_ptr`.
      2. `FlowRepository.save_snapshot(flow_id, cursor, state_ptr, metadata)`
         inserta metadata en `snapshots`.

      ## Rehidratación (worker restart)

      1. `claim_work` devuelve `flow_meta`, `latest_snapshot` (si existe) y los
         `FlowData` con `cursor > snapshot.cursor`.
      2. El consumer descarga el `state_ptr` si hay snapshot y rehidrata su
         motor externo con `snapshot_state` + `FlowData` a replay.

      ## Esquema minimalista (Postgres)

      - flows: id, current_cursor, current_version, parent_flow_id, metadata
      - steps: id, flow_id, cursor, key, payload jsonb, command_id, created_at
      - snapshots: id, flow_id, cursor, state_ptr, metadata, created_at
      - artifact_metadata: id, flow_id, key, hash, size

      ## Reglas operativas

      - Idempotencia: usar `command_id` para evitar duplicados.
      - Optimistic locking: `persist_data(..., expected_version)` usa
        `expected_version` y debe devolver `Conflict` si la versión no coincide.
      - Snapshot cada N pasos o por tamaño (decisión externa o por política).
      - Creación de ramas debe ser atómica en BD.
      - Rehidratación: snapshot + replay de `FlowData`.

      Si quieres, ahora puedo:

      1. Añadir cabeceras de archivo en todos los `.rs` del crate.
      2. Revisar `src/stubs.rs` para asegurar optimistic concurrency e idempotencia.
      3. Crear un esqueleto de `PostgresFlowRepository` con ejemplos SQL.

      Dime cuál prefieres (1 / 2 / 3) o responde "continúa" para seguir con la tarea 2.
- `FlowService` — capa de orquestación para API/operaciones atómicas.
- `InMemoryFlowRepository` — implementación de desarrollo.

## Secuencia simplificada: crear una rama desde snapshot/cursor

1. El caller (API/handler) invoca `FlowService::create_branch_from_snapshot`
   pasando `parent_flow_id`, `parent_cursor`, `name`, `status` y `metadata`.
2. `FlowService` delega en `FlowRepository.create_branch`, que debe realizar
   de forma atómica: crear la fila en `flows`, copiar los `FlowData` del
   parent hasta `parent_cursor` (si aplica) y añadir un `FlowData` tipo
   `BranchCreated` en `steps` para la nueva rama.
3. `create_branch` devuelve el nuevo `flow_id` (UUID) del branch creado.

Nota: por defecto no se copian blobs (`state_ptr`). Copiar solo si la
rama necesita aislamiento (copy-on-write).

## Snapshot: guardar estado

1. Serializar estado y subir a ObjectStore → obtener `state_ptr`.
2. `FlowRepository.save_snapshot(flow_id, cursor, state_ptr, metadata)`
   inserta metadata en `snapshots`.

## Rehidratación (worker restart)

1. `claim_work` devuelve `flow_meta`, `latest_snapshot` (si existe) y los
   `FlowData` con `cursor > snapshot.cursor`.
2. El consumer descarga el `state_ptr` si hay snapshot y rehidrata su
   motor externo con `snapshot_state` + `FlowData` a replay.

## Esquema minimalista (Postgres)

-- flows: id, current_cursor, current_version, parent_flow_id, metadata
-- steps: id, flow_id, cursor, key, payload jsonb, command_id, created_at
-- snapshots: id, flow_id, cursor, state_ptr, metadata, created_at
-- artifact_metadata: id, flow_id, key, hash, size

## Reglas operativas

- Idempotencia: usar `command_id` para evitar duplicados.
- Optimistic locking: `persist_data(..., expected_version)` usa
  `expected_version` y debe devolver `Conflict` si la versión no coincide.
- Snapshot cada N pasos o por tamaño.
- Creación de ramas debe ser atómica en BD.
- Rehidratación: snapshot + replay de `FlowData`.

Si quieres, ahora puedo:

1. Añadir cabeceras de archivo en todos los `.rs` del crate.
2. Mejorar `src/stubs.rs` para aplicar optimistic concurrency e idempotencia (si no está ya).
3. Crear un esqueleto de `PostgresFlowRepository` con SQL example.

Dime cuál prefieres (1 / 2 / 3) o responde "continúa" para seguir con la tarea 2.
