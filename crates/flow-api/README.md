# Flow-API: API RESTful para Workflows Químicos CADMA

API RESTful completa para ejecutar y gestionar workflows químicos CADMA con persistencia en PostgreSQL/SQLite.

## 🚀 Características

- **Arquitectura Hexagonal**: Separación clara entre dominio, aplicación e infraestructura
- **Persistencia Real**: Soporte para PostgreSQL (producción) y SQLite (desarrollo/tests)
- **Documentación Automática**: OpenAPI 3.0 + Swagger UI integrado
- **Validación de Datos**: Validación exhaustiva de requests con tipos seguros
- **Logging Estructurado**: Sistema de trazabilidad completo con `tracing`
- **Tests de Integración**: Suite completa de tests end-to-end

## 📋 Requisitos

- Rust 1.75+ (nightly toolchain configurado en el workspace)
- PostgreSQL 15+ (para producción)
- Python 3.11+ con RDKit (vía conda, para chem-providers)

## 🔧 Instalación

### 1. Clonar el repositorio

```bash
cd crates/flow-api
```

### 2. Configurar variables de entorno

Crear archivo `.env` en la raíz del proyecto:

```env
# Base de datos (PostgreSQL para producción)
DATABASE_URL=postgresql://admin:admin123@localhost:5432/mydatabase

# O SQLite para desarrollo
# DATABASE_URL=sqlite:///tmp/flow-chem-dev.db

# Configuración del servidor
PORT=3000
HOST=0.0.0.0
ENVIRONMENT=development

# Logging
RUST_LOG=info,flow_api=debug,axum=debug
```

### 3. Preparar base de datos PostgreSQL

```bash
# Ejecutar migraciones de Diesel
cd ../chem-persistence
diesel migration run --database-url postgresql://admin:admin123@localhost:5432/mydatabase

# O usar el script de reset
cd ../..
bash scripts/reset_database.sh
```

### 4. Compilar el proyecto

```bash
cd crates/flow-api
cargo build --release
```

## ▶️ Ejecución

### Modo desarrollo

```bash
cargo run
```

### Modo producción

```bash
cargo run --release
```

La API estará disponible en: `http://0.0.0.0:3000`

## 📚 Documentación

Una vez iniciado el servidor:

- **Swagger UI**: http://localhost:3000/docs
- **OpenAPI JSON**: http://localhost:3000/api-doc/openapi.json
- **Health Check**: http://localhost:3000/health

## 🌐 Endpoints Principales

### Gestión de Ejecuciones

| Método   | Endpoint                     | Descripción                   |
| -------- | ---------------------------- | ----------------------------- |
| `POST`   | `/api/flows/cadma/start`     | Iniciar nueva ejecución CADMA |
| `GET`    | `/api/flows/cadma`           | Listar todas las ejecuciones  |
| `GET`    | `/api/flows/cadma/{id}`      | Obtener estado de ejecución   |
| `POST`   | `/api/flows/cadma/{id}/step` | Ejecutar un paso específico   |
| `DELETE` | `/api/flows/cadma/{id}`      | Cancelar/eliminar ejecución   |

### Ejemplos de Uso

#### 1. Iniciar una nueva ejecución

```bash
curl -X POST http://localhost:3000/api/flows/cadma/start \
  -H "Content-Type: application/json" \
  -d '{
    "name": "mi-experimento-cadma",
    "metadata": {"user": "scientist1", "project": "drug-discovery"}
  }'
```

Respuesta:

```json
{
  "execution_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "current_step": 0,
  "created_at": "2025-10-13T10:30:00Z"
}
```

#### 2. Ejecutar Step1: Selección de Familia

```bash
curl -X POST http://localhost:3000/api/flows/cadma/{execution_id}/step \
  -H "Content-Type: application/json" \
  -d '{
    "step_index": 0,
    "payload": {
      "smiles": ["CCO", "c1ccccc1", "CC(=O)O"],
      "new_family_name": "alcohols-test",
      "new_family_description": "Familia de prueba con alcoholes"
    }
  }'
```

#### 3. Ejecutar Step2: Cálculo ADMETSA

```bash
curl -X POST http://localhost:3000/api/flows/cadma/{execution_id}/step \
  -H "Content-Type: application/json" \
  -d '{
    "step_index": 1,
    "payload": {
      "preferred_methods": ["Random1", "Random2"],
      "manual_values": null
    }
  }'
```

#### 4. Ejecutar Step3: Generación de Molécula Inicial

```bash
curl -X POST http://localhost:3000/api/flows/cadma/{execution_id}/step \
  -H "Content-Type: application/json" \
  -d '{
    "step_index": 2,
    "payload": {
      "method": "Manual",
      "smiles": "c1ccccc1"
    }
  }'
```

#### 5. Consultar estado de ejecución

```bash
curl http://localhost:3000/api/flows/cadma/{execution_id}
```

Respuesta:

```json
{
  "execution_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "running",
  "current_step": 3,
  "current_step_name": "ADMETSAInitialStep4",
  "steps_completed": [
    {
      "index": 0,
      "name": "FamilyReferenceStep1",
      "output": {...},
      "executed_at": "2025-10-13T10:31:00Z"
    },
    {
      "index": 1,
      "name": "ADMETSAPropertiesStep2",
      "output": {...},
      "executed_at": "2025-10-13T10:32:00Z"
    }
  ],
  "metadata": {...},
  "updated_at": "2025-10-13T10:33:00Z"
}
```

#### 6. Listar todas las ejecuciones

```bash
curl http://localhost:3000/api/flows/cadma
```

#### 7. Cancelar ejecución

```bash
curl -X DELETE http://localhost:3000/api/flows/cadma/{execution_id}
```

## 🧪 Testing

### Ejecutar todos los tests

```bash
cargo test
```

### Ejecutar solo tests de integración

```bash
cargo test --test integration_tests
```

### Ejecutar con logs detallados

```bash
RUST_LOG=debug cargo test -- --nocapture
```

## 🏗️ Arquitectura

```
flow-api/
├── src/
│   ├── main.rs              # Punto de entrada del servidor
│   ├── lib.rs               # Re-exports públicos
│   ├── config.rs            # Configuración desde env vars
│   ├── errors.rs            # Tipos de error y conversiones
│   ├── models.rs            # DTOs de request/response
│   ├── routes.rs            # Definición de rutas y OpenAPI
│   ├── handlers/            # Handlers HTTP
│   │   ├── mod.rs
│   │   └── cadma_handlers.rs
│   └── services/            # Lógica de negocio
│       ├── mod.rs
│       └── cadma_service.rs
├── tests/
│   └── integration_tests.rs # Tests end-to-end
├── Cargo.toml
└── README.md
```

### Principios de Diseño

1. **Hexagonal Architecture**:

   - Dominio puro en `chem-domain` y `flow`
   - Puertos definidos por traits
   - Adaptadores en `chem-persistence` y `flow-api`

2. **SOLID**:

   - **S**RP: Cada módulo tiene una responsabilidad única
   - **O**CP: Extensible vía traits sin modificar código existente
   - **L**SP: Implementaciones intercambiables de repositorios
   - **I**SP: Interfaces segregadas (FlowRepository, MoleculeReader, etc.)
   - **D**IP: Dependencias sobre abstracciones (traits), no sobre concretos

3. **Separación de Capas**:
   - `handlers/`: Capa de presentación HTTP
   - `services/`: Lógica de aplicación
   - `models/`: DTOs de transferencia
   - Dominio en crates externos

## 🐳 Docker

### Construir imagen

```bash
docker build -t flow-api:latest .
```

### Ejecutar con Docker Compose

```bash
# Desde raíz del proyecto
docker-compose up flow-api
```

## 📝 Workflow CADMA Completo

El workflow CADMA consta de 6 pasos secuenciales:

1. **Step1 - FamilyReferenceStep1**: Selección o creación de familia de moléculas
2. **Step2 - ADMETSAPropertiesStep2**: Cálculo de propiedades ADMETSA para la familia
3. **Step3 - MoleculeInitialStep3**: Generación de molécula inicial (manual o random)
4. **Step4 - ADMETSAInitialStep4**: Cálculo de propiedades para molécula inicial
5. **Step5 - SubstituteGenerationStep5**: Generación de moléculas sustituidas
6. **Step6 - ADMETSAGeneratedStep6**: Cálculo de propiedades para moléculas generadas

Cada paso persiste su resultado en la base de datos y puede ser ejecutado independientemente vía API.

## 🔍 Troubleshooting

### Error: "Error inicializando flow repository"

**Solución**: Verificar que la base de datos esté corriendo y `DATABASE_URL` sea correcta.

```bash
# Verificar conexión PostgreSQL
psql -h localhost -U admin -d mydatabase

# O verificar archivo SQLite
sqlite3 /tmp/flow-chem-dev.db ".tables"
```

### Error: "RDKit not found"

**Solución**: Instalar RDKit vía conda:

```bash
conda install -c conda-forge rdkit python=3.11
```

### Error: "Port already in use"

**Solución**: Cambiar puerto en `.env` o matar proceso existente:

```bash
# Encontrar proceso en puerto 3000
lsof -i :3000

# Matar proceso
kill -9 <PID>
```

## 🤝 Contribuciones

Este proyecto sigue estándares de Clean Architecture y SOLID. Al contribuir:

1. Mantén la separación de capas
2. Usa traits para abstracciones
3. Documenta APIs con `utoipa`
4. Escribe tests para nuevos endpoints
5. Sigue el estilo Rust estándar (`cargo fmt`)

## 📄 Licencia

MIT License - Ver LICENSE para más detalles.

## 🔗 Enlaces

- [Documentación Flow-Chem](../README.md)
- [Arquitectura del Sistema](../../refactorizacion.md)
- [CADMA Workflow Example](../chem-workflow/examples/cadma_example.rs)
