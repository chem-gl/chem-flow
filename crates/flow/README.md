# Flow Crate - Sistema de Árbol de Flujos

[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

Un sistema de gestión de flujos basado en Event Sourcing que implementa una estructura de árbol dirigido acíclico (DAG) similar a un sistema de control de versiones simplificado como Git.

## 🌟 Características Principales

- **Estructura de Árbol**: Sistema de flujos organizados como árbol dirigido acíclico (DAG)
- **Event Sourcing**: Estado inmutable reconstituido a partir de eventos ordenados
- **Sin Ciclos**: Arquitectura que previene bucles por diseño
- **Sin Merges**: Operaciones simples sin fusión de ramas
- **Sin Duplicaciones**: Verificación global de unicidad de contenido
- **Eliminación Recursiva**: Borrado automático de subramas dependientes
- **Snapshots**: Optimización de rendimiento mediante puntos de control
- **Concurrencia Optimista**: Control de versiones para operaciones concurrentes

## 🏗️ Arquitectura

### Arquitectura Hexagonal

El proyecto sigue los principios de arquitectura hexagonal (puertos y adaptadores):

```text
┌─────────────────────────────────────────────────────────────┐
│                        Aplicación                           │
│  ┌─────────────────┐ ┌─────────────────┐ ┌──────────────┐   │
│  │ Casos de Uso    │ │ Servicios App   │ │ Comandos     │   │
│  └─────────────────┘ └─────────────────┘ └──────────────┘   │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────┼───────────────────────────────────┐
│                    Dominio (Core)                           │
│  ┌─────────────────┐    │    ┌─────────────────┐            │
│  │ Entidades       │    │    │ Value Objects   │            │
│  │ - FlowNode      │    │    │ - FlowData      │            │
│  │ - FlowTree      │    │    │ - SnapshotMeta  │            │
│  │ - Branch        │    │    │ - FlowMeta      │            │
│  └─────────────────┘    │    └─────────────────┘            │
│                         │                                   │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                    Puertos                              │ │
│  │ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────┐ │ │
│  │ │ FlowRepository  │ │ SnapshotStore   │ │ EventStore  │ │ │
│  │ └─────────────────┘ └─────────────────┘ └─────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────┼───────────────────────────────────┐
│                   Adaptadores                               │
│  ┌─────────────────┐    │    ┌─────────────────┐            │
│  │ InMemoryRepo    │    │    │ PostgresRepo    │            │
│  │ S3SnapshotStore │    │    │ FileSystemStore │            │
│  └─────────────────┘    │    └─────────────────┘            │
└─────────────────────────┴───────────────────────────────────┘
```

### Principios SOLID Aplicados

1. **Single Responsibility Principle (SRP)**: Cada módulo tiene una responsabilidad específica
2. **Open/Closed Principle (OCP)**: Extensible mediante nuevos adaptadores sin modificar el core
3. **Liskov Substitution Principle (LSP)**: Implementaciones intercambiables de repositorios
4. **Interface Segregation Principle (ISP)**: Interfaces específicas para cada necesidad
5. **Dependency Inversion Principle (DIP)**: Dependencias hacia abstracciones, no implementaciones

## 🌳 Algoritmo del Árbol de Flujos

### Conceptos Fundamentales

El sistema implementa una estructura de árbol donde:

- **Nodo**: Representa un paso individual con contenido único
- **Rama (Branch)**: Vista lineal del árbol desde la raíz hasta un nodo específico  
- **Flujo Principal**: La rama inicial del sistema
- **Subramas**: Ramas que divergen desde un punto específico de otra rama

### Estructura de Datos

```rust
// Nodo del árbol
struct FlowNode {
    id: Uuid,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    content: FlowData,
}

// Árbol completo
struct FlowTree {
    nodes: HashMap<NodeId, FlowNode>,
    branches: HashMap<BranchId, BranchMeta>,
    root: NodeId,
}

// Metadata de rama
struct BranchMeta {
    head: NodeId,          // Último nodo de la rama
    start_node: NodeId,    // Nodo donde inicia la rama
    parent_branch: Option<BranchId>,
    children_branches: Vec<BranchId>,
}
```

### Operaciones Principales

#### 1. Añadir Paso (`add_step`)

```rust
fn add_step(branch_id: &BranchId, content: FlowData) -> Result<()> {
    // 1. Verificar duplicación global
    if content_exists_globally(&content) {
        return Err("Duplicación no permitida");
    }
    
    // 2. Crear nuevo nodo
    let new_node = FlowNode::new(parent: branch.head, content);
    
    // 3. Actualizar referencias
    nodes[branch.head].children.push(new_node.id);
    branches[branch_id].head = new_node.id;
    
    Ok(())
}
```

#### 2. Crear Rama (`create_branch`)

```rust
fn create_branch(from_branch: &BranchId, at_step: usize) -> Result<BranchId> {
    // 1. Obtener path de la rama padre
    let path = get_path(from_branch)?;
    
    // 2. Validar punto de ramificación
    if at_step > path.len() {
        return Err("Paso inválido");
    }
    
    // 3. Crear nueva rama apuntando al nodo de ramificación
    let start_node = path[at_step - 1];
    let new_branch = BranchMeta {
        head: start_node,
        start_node,
        parent_branch: Some(from_branch),
        children_branches: vec![],
    };
    
    // 4. Actualizar relaciones padre-hijo
    branches[from_branch].children_branches.push(new_branch_id);
    
    Ok(new_branch_id)
}
```

#### 3. Eliminar Rama (`delete_branch`)

```rust
fn delete_branch(branch_id: &BranchId) -> Result<()> {
    // 1. Eliminar subramas recursivamente
    for child_branch in &branches[branch_id].children_branches {
        delete_branch(child_branch)?;
    }
    
    // 2. Eliminar nodos exclusivos de la rama
    let mut current = branches[branch_id].head;
    let start_node = branches[branch_id].start_node;
    
    while current != start_node {
        let parent = nodes[current].parent.unwrap();
        nodes[parent].children.retain(|&x| x != current);
        nodes.remove(current);
        current = parent;
    }
    
    // 3. Actualizar rama padre
    if let Some(parent_branch) = branches[branch_id].parent_branch {
        branches[parent_branch].children_branches.retain(|&x| x != branch_id);
    }
    
    // 4. Eliminar rama
    branches.remove(branch_id);
    
    Ok(())
}
```

#### 4. Obtener Path (`get_path`)

```rust
fn get_path(branch_id: &BranchId) -> Result<Vec<NodeId>> {
    let mut path = Vec::new();
    let mut current = Some(branches[branch_id].head);
    
    // Construir path hacia atrás usando pila
    while let Some(node_id) = current {
        path.push(node_id);
        current = nodes[node_id].parent;
    }
    
    // Invertir para tener raíz primero
    path.reverse();
    Ok(path)
}
```

### Garantías del Sistema

1. **Sin Ciclos**: Cada nodo tiene exactamente un padre (excepto la raíz)
2. **Sin Merges**: No se permite unir ramas divergentes
3. **Sin Duplicaciones**: Verificación global de contenido único
4. **Consistencia**: Eliminación recursiva preserva integridad

## 🚀 Uso Básico

### Configuración

```rust
use flow::{
    FlowTreeSystem, 
    InMemoryRepository, 
    FlowData
};

// Crear sistema con repositorio en memoria
let repo = Arc::new(InMemoryRepository::new());
let flow_system = FlowTreeSystem::new(repo);
```

### Crear Flujo Principal

```rust
// Crear flujo principal
let main_flow = flow_system.create_flow(
    Some("Flujo Principal".to_string()),
    Some("active".to_string()),
    json!({})
)?;

// Añadir pasos secuenciales
for i in 1..=5 {
    flow_system.add_step(
        &main_flow,
        &format!("Paso {}", i),
        json!({"description": format!("Descripción del paso {}", i)})
    )?;
}
```

### Crear y Gestionar Ramas

```rust
// Crear rama desde el paso 3
let branch_a = flow_system.create_branch(
    &main_flow,
    3, // punto de ramificación
    json!({"purpose": "explorar alternativa A"})
)?;

// Añadir pasos a la rama
flow_system.add_step(
    &branch_a,
    "Paso alternativo 4A",
    json!({"alternative": "A", "step": 4})
)?;

// Crear subrama desde rama A
let sub_branch = flow_system.create_branch(
    &branch_a,
    2, // desde paso 2 de la rama A
    json!({"purpose": "sub-experimento"})
)?;
```

### Consultar Estado

```rust
// Obtener path completo de una rama
let path = flow_system.get_branch_path(&branch_a)?;
println!("Rama A tiene {} pasos", path.len());

// Obtener metadata
let meta = flow_system.get_flow_meta(&branch_a)?;
println!("Estado: {:?}", meta.status);

// Contar pasos
let step_count = flow_system.count_steps(&main_flow)?;
println!("Flujo principal: {} pasos", step_count);
```

### Eliminar Ramas

```rust
// Eliminar rama (incluye todas las subramas automáticamente)
flow_system.delete_branch(&branch_a)?;

// Eliminar desde un paso específico
flow_system.delete_from_step(&main_flow, 3)?; // elimina pasos 3 en adelante
```

## 📚 API Reference

### Core Types

```rust
pub struct FlowData {
    pub id: Uuid,
    pub flow_id: Uuid,
    pub cursor: i64,           // Posición secuencial (1, 2, 3, ...)
    pub key: String,           // Clave semántica del evento
    pub payload: Value,        // Contenido principal
    pub metadata: Value,       // Metadata adicional
    pub command_id: Option<Uuid>, // Para idempotencia
    pub created_at: DateTime<Utc>,
}

pub struct FlowMeta {
    pub id: Uuid,
    pub name: Option<String>,
    pub status: Option<String>,
    pub current_cursor: i64,
    pub current_version: i64,
    pub parent_flow_id: Option<Uuid>,
    pub parent_cursor: Option<i64>,
    pub metadata: Value,
}
```

### Repository Interface

```rust
pub trait FlowRepository: Send + Sync {
    fn create_flow(&self, name: Option<String>, status: Option<String>, metadata: Value) -> Result<Uuid>;
    fn get_flow_meta(&self, flow_id: &Uuid) -> Result<FlowMeta>;
    fn persist_data(&self, data: &FlowData, expected_version: i64) -> Result<PersistResult>;
    fn read_data(&self, flow_id: &Uuid, from_cursor: i64) -> Result<Vec<FlowData>>;
    fn create_branch(&self, parent_flow_id: &Uuid, parent_cursor: i64, metadata: Value) -> Result<Uuid>;
    fn delete_branch(&self, flow_id: &Uuid) -> Result<()>;
    fn delete_from_step(&self, flow_id: &Uuid, from_cursor: i64) -> Result<()>;
    
    // Snapshots
    fn save_snapshot(&self, flow_id: &Uuid, cursor: i64, state_ptr: &str, metadata: Value) -> Result<Uuid>;
    fn load_latest_snapshot(&self, flow_id: &Uuid) -> Result<Option<SnapshotMeta>>;
    
    // Metadata operations
    fn get_meta(&self, flow_id: &Uuid, key: &str) -> Result<Value>;
    fn set_meta(&self, flow_id: &Uuid, key: &str, value: Value) -> Result<()>;
    fn del_meta(&self, flow_id: &Uuid, key: &str) -> Result<()>;
    
    // Utility
    fn list_flow_ids(&self) -> Result<Vec<Uuid>>;
    fn count_steps(&self, flow_id: &Uuid) -> Result<i64>;
    fn branch_exists(&self, flow_id: &Uuid) -> Result<bool>;
}
```

## 🔧 Ejemplos Avanzados

### Ejemplo 1: Flujo de Experimentación Química

```rust
use flow::*;

// Crear experimento principal
let experiment = system.create_flow(
    Some("Síntesis Compuesto X".to_string()),
    Some("planning".to_string()),
    json!({
        "compound": "X",
        "target_yield": 0.85,
        "safety_level": "high"
    })
)?;

// Pasos iniciales
system.add_step(&experiment, "preparation", json!({
    "materials": ["reactivo_a", "reactivo_b", "catalizador"],
    "temperature": 25,
    "pressure": "1 atm"
}))?;

system.add_step(&experiment, "mixing", json!({
    "method": "magnetic_stirring",
    "duration_min": 30,
    "speed_rpm": 300
}))?;

// Crear rama para probar temperatura alternativa
let temp_variant = system.create_branch(&experiment, 1, json!({
    "hypothesis": "Mayor temperatura mejora rendimiento",
    "variable": "temperature"
}))?;

system.add_step(&temp_variant, "heating", json!({
    "target_temp": 60,
    "heating_rate": "5°C/min"
}))?;

system.add_step(&temp_variant, "reaction", json!({
    "duration_min": 120,
    "monitoring": ["temperature", "pressure", "color_change"]
}))?;

// Crear rama para catalizador alternativo
let catalyst_variant = system.create_branch(&experiment, 1, json!({
    "hypothesis": "Catalizador B es más eficiente",
    "variable": "catalyst"
}))?;

system.add_step(&catalyst_variant, "catalyst_preparation", json!({
    "catalyst": "catalyst_b",
    "activation_temp": 200,
    "activation_time_min": 45
}))?;
```

### Ejemplo 2: Pipeline de Análisis de Datos

```rust
// Pipeline principal de procesamiento
let pipeline = system.create_flow(
    Some("Análisis Dataset Experimentos".to_string()),
    Some("active".to_string()),
    json!({
        "dataset_size": 50000,
        "analysis_type": "exploratory"
    })
)?;

// Pasos de preprocessing
system.add_step(&pipeline, "load_data", json!({
    "source": "experiments_2024.csv",
    "encoding": "utf-8",
    "separator": ","
}))?;

system.add_step(&pipeline, "clean_data", json!({
    "remove_duplicates": true,
    "handle_missing": "interpolate",
    "outlier_detection": "iqr"
}))?;

system.add_step(&pipeline, "feature_engineering", json!({
    "scaling": "standard",
    "encoding": "one_hot",
    "feature_selection": "correlation"
}))?;

// Rama para modelo de regresión
let regression_branch = system.create_branch(&pipeline, 3, json!({
    "model_type": "regression",
    "target": "yield_percentage"
}))?;

system.add_step(&regression_branch, "train_regression", json!({
    "algorithm": "random_forest",
    "cross_validation": 5,
    "hyperparameters": {
        "n_estimators": 100,
        "max_depth": 10
    }
}))?;

// Rama para modelo de clasificación
let classification_branch = system.create_branch(&pipeline, 3, json!({
    "model_type": "classification",
    "target": "success_category"
}))?;

system.add_step(&classification_branch, "train_classification", json!({
    "algorithm": "gradient_boosting",
    "cross_validation": 5,
    "hyperparameters": {
        "learning_rate": 0.1,
        "n_estimators": 200
    }
}))?;
```

### Ejemplo 3: Uso con Snapshots para Optimización

```rust
// Crear flujo con muchos pasos
let long_flow = system.create_flow(
    Some("Procesamiento Largo".to_string()),
    Some("active".to_string()),
    json!({})
)?;

// Añadir 100 pasos
for i in 1..=100 {
    system.add_step(&long_flow, &format!("step_{}", i), json!({
        "step_number": i,
        "data": format!("processed_data_{}", i)
    }))?;
    
    // Crear snapshot cada 20 pasos para optimización
    if i % 20 == 0 {
        let state = json!({
            "checkpoint": i,
            "accumulated_results": format!("results_up_to_{}", i)
        });
        
        system.save_snapshot(&long_flow, i, &format!("checkpoint_{}", i), state)?;
    }
}

// Rehidratar desde snapshot más reciente
let latest_snapshot = system.load_latest_snapshot(&long_flow)?;
if let Some(snapshot) = latest_snapshot {
    println!("Rehidratando desde cursor: {}", snapshot.cursor);
    
    // Leer solo eventos posteriores al snapshot
    let remaining_steps = system.read_data(&long_flow, snapshot.cursor)?;
    println!("Aplicando {} pasos adicionales", remaining_steps.len());
}
```

## 🧪 Testing

### Ejecutar Tests

```bash
# Todos los tests
cargo test

# Tests específicos
cargo test flow_tree_operations
cargo test branching_tree
cargo test inmemory_behavior

# Tests con output detallado
cargo test -- --nocapture
```

### Tests de Integración

El proyecto incluye tests exhaustivos que verifican:

- ✅ Creación y gestión de flujos principales
- ✅ Ramificación desde puntos específicos
- ✅ Evolución independiente de ramas
- ✅ Eliminación recursiva de subramas
- ✅ Prevención de duplicaciones
- ✅ Concurrencia optimista
- ✅ Operaciones de snapshots
- ✅ Rehidratación desde checkpoints
- ✅ Metadata y operaciones de estado

## 🔄 Casos de Uso

### 1. Investigación Científica

- **Experimentación Química**: Probar diferentes condiciones de reacción
- **Análisis de Datos**: Explorar diferentes pipelines de procesamiento
- **Simulaciones**: Modelar escenarios alternativos

### 2. Desarrollo de Software

- **Feature Branches**: Desarrollo de características independientes
- **A/B Testing**: Comparar implementaciones alternativas
- **Rollback Seguro**: Volver a estados estables conocidos

### 3. Procesos de Negocio

- **Flujos de Aprobación**: Gestionar diferentes rutas de autorización
- **Auditoría**: Trazabilidad completa de decisiones y cambios
- **Optimización**: Probar mejoras sin afectar procesos principales

## 📊 Rendimiento

### Características de Rendimiento

- **Event Sourcing**: O(n) para reconstruir estado desde eventos
- **Snapshots**: O(1) para acceso a checkpoints + O(k) para replay incremental
- **Ramificación**: O(1) para crear ramas (compartición de nodos)
- **Consultas**: O(log n) para búsquedas por ID, O(d) para paths (d = profundidad)

### Optimizaciones Implementadas

1. **Snapshots Periódicos**: Reducen tiempo de rehidratación
2. **Compartición de Nodos**: Ramas comparten historia común
3. **Lazy Loading**: Carga bajo demanda de datos grandes
4. **Indexación**: Acceso eficiente por claves semánticas

## 🤝 Contribuir

### Desarrollo Local

```bash
# Clonar repositorio
git clone <repository-url>
cd flow-chem/crates/flow

# Ejecutar tests
cargo test

# Verificar estilo
cargo fmt --check
cargo clippy -- -D warnings

# Documentación
cargo doc --open
```

### Guidelines

1. **Seguir principios SOLID**: Cada cambio debe mantener la arquitectura limpia
2. **Tests exhaustivos**: Toda nueva funcionalidad debe incluir tests
3. **Documentación**: Actualizar README y documentación inline
4. **Compatibilidad**: Mantener backward compatibility cuando sea posible

## 📝 Licencia

MIT License - ver [LICENSE](LICENSE) para detalles.

## 🙏 Agradecimientos

Este proyecto está inspirado en sistemas de control de versiones como Git y en principios de Event Sourcing para crear un sistema robusto de gestión de flujos científicos y de experimentación.
