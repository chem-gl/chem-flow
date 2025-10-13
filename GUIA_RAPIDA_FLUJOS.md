# Guía Rápida: Sistema de Árbol de Flujos

## 🚀 Inicio Rápido

### Ejecutar Tests

```bash
# Tests en memoria (20 tests)
cargo test -p flow --test flow_tree_operations

# Tests con Diesel SQLite (13 tests)
cargo test -p chem-persistence --test flow_tree_diesel_integration

# Todos los tests del sistema de flujos
cargo test -p flow -p chem-persistence --test flow_tree_operations --test flow_tree_diesel_integration
```

### Ejecutar Ejemplo Completo

```bash
cargo run --example flow_tree_complete_example
```

---

## 📚 Uso Básico

### 1. Crear Repositorio

```rust
use flow::stubs::InMemoryFlowRepository;
use std::sync::Arc;

// Para tests y desarrollo
let repo = Arc::new(InMemoryFlowRepository::new());

// Para producción (requiere DATABASE_URL)
use chem_persistence::new_flow_from_env;
let repo = Arc::new(new_flow_from_env().expect("db"));
```

### 2. Crear Flujo Principal

```rust
use serde_json::json;

let flow_id = repo.create_flow(
    Some("Mi Experimento".into()),
    Some("active".into()),
    json!({"type": "synthesis", "version": "1.0"})
).expect("crear flujo");
```

### 3. Añadir Pasos

```rust
use flow::domain::{FlowData, PersistResult};
use chrono::Utc;
use uuid::Uuid;

// Helper para añadir pasos
fn append_step(repo: &dyn FlowRepository, flow_id: &Uuid, content: &str) {
    let meta = repo.get_flow_meta(flow_id).expect("meta");
    let next_cursor = meta.current_cursor + 1;

    let step = FlowData {
        id: Uuid::new_v4(),
        flow_id: *flow_id,
        cursor: next_cursor,
        key: format!("step_{}", next_cursor),
        payload: json!({"content": content}),
        metadata: json!({}),
        command_id: None,
        created_at: Utc::now(),
    };

    let result = repo.persist_data(&step, meta.current_version).expect("persist");
    assert!(matches!(result, PersistResult::Ok { .. }));
}

// Usar el helper
append_step(&*repo, &flow_id, "Preparar reactivos");
append_step(&*repo, &flow_id, "Mezclar componentes");
```

### 4. Crear Rama

```rust
// Crear rama desde paso 5
let branch_id = repo.create_branch(
    &flow_id,
    5,
    json!({"reason": "explorar método alternativo"})
).expect("crear rama");

// La rama hereda pasos 1-5 automáticamente
// Continuar con pasos nuevos
append_step(&*repo, &branch_id, "Paso alternativo 6");
```

### 5. Guardar Snapshots

```rust
// Guardar checkpoint
repo.save_snapshot(
    &flow_id,
    10,
    "snapshot_step_10",
    json!({"description": "checkpoint después de preparación"})
).expect("guardar snapshot");
```

### 6. Rehidratar Estado

```rust
// Cargar último snapshot
let snap_opt = repo.load_latest_snapshot(&flow_id).expect("cargar");

if let Some(snap) = snap_opt {
    // Replay pasos posteriores
    let replay = repo.read_data(&flow_id, snap.cursor).expect("replay");

    for step in replay {
        // Aplicar paso al estado del motor
        engine.apply_step(&step);
    }
}
```

### 7. Eliminar Rama

```rust
// Eliminar rama (no afecta al padre)
repo.delete_branch(&branch_id).expect("eliminar");

// Verificar
assert!(!repo.branch_exists(&branch_id).unwrap());
assert!(repo.branch_exists(&flow_id).unwrap()); // Padre intacto
```

---

## 🔍 Operaciones Útiles

### Listar Todos los Flujos

```rust
let all_ids = repo.list_flow_ids().expect("listar");

for id in all_ids {
    let meta = repo.get_flow_meta(&id).expect("meta");
    println!("{}: {} pasos",
             meta.name.unwrap_or_default(),
             meta.current_cursor);
}
```

### Contar Pasos

```rust
let count = repo.count_steps(&flow_id).expect("contar");
println!("El flujo tiene {} pasos", count);
```

### Verificar Existencia

```rust
if repo.branch_exists(&flow_id).expect("check") {
    println!("El flujo existe");
}
```

### Metadata

```rust
// Establecer metadata
repo.set_meta(&flow_id, "workflow_type", json!("CADMA")).unwrap();

// Obtener metadata
let wf_type = repo.get_meta(&flow_id, "workflow_type").unwrap();

// Eliminar metadata
repo.del_meta(&flow_id, "workflow_type").unwrap();
```

### Status del Flujo

```rust
// Actualizar status
repo.set_flow_status(&flow_id, Some("completed".into())).unwrap();

// Obtener status
let status = repo.get_flow_status(&flow_id).unwrap();
```

---

## 🎯 Patrones Comunes

### Patrón: Exploración de Alternativas

```rust
// Flujo principal hasta paso N
for i in 1..=10 {
    append_step(&*repo, &main_id, &format!("Paso {}", i));
}

// Explorar alternativa A desde paso 5
let alt_a = repo.create_branch(&main_id, 5, json!({"method": "A"})).unwrap();
append_step(&*repo, &alt_a, "Método A - paso 6");

// Explorar alternativa B desde paso 5
let alt_b = repo.create_branch(&main_id, 5, json!({"method": "B"})).unwrap();
append_step(&*repo, &alt_b, "Método B - paso 6");

// Comparar resultados y eliminar alternativa descartada
repo.delete_branch(&alt_b).unwrap();
```

### Patrón: Checkpoints Periódicos

```rust
for i in 1..=100 {
    append_step(&*repo, &flow_id, &format!("Paso {}", i));

    // Snapshot cada 10 pasos
    if i % 10 == 0 {
        repo.save_snapshot(
            &flow_id,
            i,
            &format!("snap_{}", i),
            json!({"step": i})
        ).unwrap();
    }
}
```

### Patrón: Árbol de Decisiones

```rust
// Nivel 1: Flujo principal
let main = repo.create_flow(Some("main".into()), None, json!({})).unwrap();
for i in 1..=10 { append_step(&*repo, &main, &format!("M{}", i)); }

// Nivel 2: Ramas principales
let b1 = repo.create_branch(&main, 5, json!({})).unwrap();
let b2 = repo.create_branch(&main, 5, json!({})).unwrap();

// Nivel 3: Subramas
let b1_1 = repo.create_branch(&b1, 7, json!({})).unwrap();
let b1_2 = repo.create_branch(&b1, 7, json!({})).unwrap();
```

---

## ⚠️ Consideraciones Importantes

### Locking Optimista

```rust
// SIEMPRE usar la versión actual
let meta = repo.get_flow_meta(&flow_id).unwrap();
let result = repo.persist_data(&step, meta.current_version); // ✅ Correcto

// NO usar versión hardcoded
let result = repo.persist_data(&step, 5); // ❌ Puede causar conflicto
```

### Cursors Secuenciales

```rust
// El cursor siempre debe ser current_cursor + 1
let meta = repo.get_flow_meta(&flow_id).unwrap();
step.cursor = meta.current_cursor + 1; // ✅ Correcto

// NO saltar cursors
step.cursor = meta.current_cursor + 5; // ❌ Rompe secuencia
```

### Eliminación de Ramas

```rust
// Las ramas se eliminan completamente
repo.delete_branch(&branch_id).unwrap();

// NO se puede recuperar después
// Guardar snapshot antes si necesitas preservar
```

---

## 📖 Documentación Completa

- **Verificación Técnica**: `FLOW_TREE_VERIFICATION.md`
- **Resumen Ejecutivo**: `RESUMEN_VERIFICACION_FLUJOS.md`
- **Ejemplo Completo**: `examples/flow_tree_complete_example.rs`
- **Tests**:
  - `crates/flow/tests/flow_tree_operations.rs`
  - `crates/chem-persistence/tests/flow_tree_diesel_integration.rs`

---

## ✅ Tests de Verificación

```bash
# Verificar que todo funciona
cargo test -p flow -p chem-persistence \
    --test flow_tree_operations \
    --test flow_tree_diesel_integration

# Resultado esperado: 33 tests PASSED (20 + 13)
```

---

**Sistema verificado y listo para uso en producción** ✅
