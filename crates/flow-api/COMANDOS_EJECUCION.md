# Comandos para Ejecutar Flow-API

Este documento contiene TODOS los comandos que debes ejecutar para compilar, configurar y ejecutar la API CADMA.

## 📋 Prerrequisitos

Asegúrate de tener:

- PostgreSQL corriendo en localhost:5432
- Variables de entorno configuradas en `.env`
- Python 3.11+ con RDKit instalado

## 🚀 Comandos de Ejecución

### 1. Dar permisos de ejecución al script

```bash
chmod +x scripts/run_flow_api.sh
```

### 2. Configurar base de datos PostgreSQL (si no está ya corriendo)

```bash
# Iniciar PostgreSQL con Docker Compose
docker-compose up -d db

# Esperar a que esté listo (unos 10 segundos)
sleep 10

# Aplicar migraciones
cd crates/chem-persistence
diesel migration run --database-url postgresql://admin:admin123@localhost:5432/mydatabase
cd ../..
```

### 3. Compilar el proyecto flow-api

```bash
cd crates/flow-api
cargo build --release
```

### 4. Ejecutar tests (opcional pero recomendado)

```bash
# Tests unitarios y de integración
cargo test

# O con logs detallados
RUST_LOG=debug cargo test -- --nocapture
```

### 5. Ejecutar la API

#### Opción A: Usando el script (RECOMENDADO)

```bash
# Desde crates/flow-api
../../scripts/run_flow_api.sh
```

#### Opción B: Usando cargo run directamente

```bash
# Desde crates/flow-api
cargo run --release
```

#### Opción C: Ejecutando el binario compilado

```bash
# Desde la raíz del proyecto
./target/release/flow-api
```

### 6. Verificar que la API está funcionando

En otra terminal:

```bash
# Health check
curl http://localhost:3000/health

# Debería responder:
# {"message":"API CADMA operativa"}
```

### 7. Abrir Swagger UI en el navegador

```bash
# En Linux
xdg-open http://localhost:3000/docs

# En macOS
open http://localhost:3000/docs

# O simplemente navega a: http://localhost:3000/docs
```

## 🧪 Comandos de Testing

### Test completo del workflow CADMA

```bash
# 1. Iniciar nueva ejecución
EXECUTION_ID=$(curl -s -X POST http://localhost:3000/api/flows/cadma/start \
  -H "Content-Type: application/json" \
  -d '{"name":"test-flow","metadata":{}}' | jq -r '.execution_id')

echo "Ejecución creada: $EXECUTION_ID"

# 2. Ejecutar Step1 (crear familia con SMILES)
curl -X POST "http://localhost:3000/api/flows/cadma/$EXECUTION_ID/step" \
  -H "Content-Type: application/json" \
  -d '{
    "step_index": 0,
    "payload": {
      "smiles": ["CCO", "c1ccccc1"],
      "new_family_name": "test-family"
    }
  }'

# 3. Consultar estado
curl "http://localhost:3000/api/flows/cadma/$EXECUTION_ID" | jq

# 4. Ejecutar Step2 (ADMETSA)
curl -X POST "http://localhost:3000/api/flows/cadma/$EXECUTION_ID/step" \
  -H "Content-Type: application/json" \
  -d '{
    "step_index": 1,
    "payload": {
      "preferred_methods": ["Random1", "Random2"]
    }
  }'

# 5. Ejecutar Step3 (Molécula inicial)
curl -X POST "http://localhost:3000/api/flows/cadma/$EXECUTION_ID/step" \
  -H "Content-Type: application/json" \
  -d '{
    "step_index": 2,
    "payload": {
      "method": "Manual",
      "smiles": "c1ccccc1"
    }
  }'

# 6. Listar todas las ejecuciones
curl http://localhost:3000/api/flows/cadma | jq

# 7. Cancelar ejecución
curl -X DELETE "http://localhost:3000/api/flows/cadma/$EXECUTION_ID" | jq
```

## 🐳 Comandos Docker (Alternativa)

Si prefieres ejecutar todo con Docker:

```bash
# Construir y ejecutar con docker-compose
docker-compose up --build flow-api

# O solo la API (asumiendo que db ya está corriendo)
docker-compose up flow-api

# Ver logs
docker-compose logs -f flow-api

# Detener
docker-compose down
```

## 🔧 Comandos de Desarrollo

### Formatear código

```bash
cd crates/flow-api
cargo fmt
```

### Verificar lints

```bash
cargo clippy -- -D warnings
```

### Limpiar build artifacts

```bash
cargo clean
```

### Actualizar dependencias

```bash
cargo update
```

### Ver árbol de dependencias

```bash
cargo tree
```

## 📊 Monitoreo y Logs

### Ver logs en tiempo real

```bash
# Con nivel debug
RUST_LOG=debug cargo run --release 2>&1 | tee api.log

# Solo errores
RUST_LOG=error cargo run --release
```

### Analizar logs guardados

```bash
# Ver últimas 50 líneas
tail -n 50 api.log

# Filtrar errores
grep -i error api.log

# Seguir logs en vivo
tail -f api.log
```

## 🛠️ Troubleshooting

### Si el puerto 3000 está ocupado

```bash
# Cambiar puerto en .env
echo "PORT=3001" >> .env

# O exportar variable
export PORT=3001
cargo run --release
```

### Si PostgreSQL no está accesible

```bash
# Verificar estado
docker-compose ps db

# Reiniciar
docker-compose restart db

# O usar SQLite para tests
export DATABASE_URL="sqlite:///tmp/flow-chem-test.db"
cargo run --release
```

### Si faltan migraciones

```bash
cd crates/chem-persistence

# Re-ejecutar todas las migraciones
diesel migration redo --database-url postgresql://admin:admin123@localhost:5432/mydatabase

# O usar el script de reset
cd ../..
bash scripts/reset_database.sh
```

## 📦 Comandos de Instalación de Dependencias (Sistema)

Si necesitas instalar dependencias del sistema:

### Ubuntu/Debian

```bash
sudo apt-get update
sudo apt-get install -y \
  postgresql-client \
  libpq-dev \
  pkg-config \
  libssl-dev \
  build-essential
```

### macOS

```bash
brew install postgresql libpq openssl pkg-config
```

### RDKit (vía Conda)

```bash
# Instalar conda si no lo tienes
wget https://repo.anaconda.com/miniconda/Miniconda3-latest-Linux-x86_64.sh
bash Miniconda3-latest-Linux-x86_64.sh

# Instalar RDKit
conda install -c conda-forge rdkit python=3.11
```

## 🎯 Comandos Rápidos (Quick Start)

Para iniciar rápidamente (asumiendo que todo está instalado):

```bash
# Todo en uno
cd crates/flow-api && \
  cargo build --release && \
  ../../scripts/run_flow_api.sh
```

## 📝 Notas Importantes

1. **Primer inicio**: La primera compilación puede tardar varios minutos
2. **Base de datos**: Asegúrate de que PostgreSQL esté corriendo antes de iniciar
3. **Python/RDKit**: Necesario para chem-providers (cálculos químicos)
4. **Migraciones**: Se aplican automáticamente con el script run_flow_api.sh
5. **Tests**: Usan SQLite en memoria, no requieren PostgreSQL

## ✅ Verificación Final

Ejecuta estos comandos para verificar que todo funciona:

```bash
# 1. Health check
curl http://localhost:3000/health

# 2. OpenAPI spec
curl http://localhost:3000/api-doc/openapi.json | jq

# 3. Crear y consultar ejecución
EXEC_ID=$(curl -s -X POST http://localhost:3000/api/flows/cadma/start \
  -H "Content-Type: application/json" \
  -d '{"name":"verification-test"}' | jq -r '.execution_id')

curl "http://localhost:3000/api/flows/cadma/$EXEC_ID" | jq

# Si todos responden correctamente, ¡la API está lista! ✅
```

## 🎉 ¡Listo!

La API Flow-Chem CADMA está completamente operativa en http://localhost:3000

- Documentación: http://localhost:3000/docs
- Pruebas interactivas: Swagger UI
- Endpoints: Ver README.md para ejemplos completos
