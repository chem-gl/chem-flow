#!/bin/bash

# Script para ejecutar la API RESTful de CADMA
# Uso: ./scripts/run_flow_api.sh [--build] [--docker]

set -e

# Colores para output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 Iniciando API RESTful de CADMA${NC}"
echo ""

# Verificar modo de ejecución
DOCKER_MODE=false
BUILD_MODE=false

for arg in "$@"; do
    case $arg in
        --docker)
            DOCKER_MODE=true
            ;;
        --build)
            BUILD_MODE=true
            ;;
    esac
done

# Modo Docker Compose
if [ "$DOCKER_MODE" = true ]; then
    echo -e "${BLUE}🐳 Ejecutando en modo Docker Compose${NC}"
    
    # Verificar que existe docker-compose
    if ! command -v docker-compose &> /dev/null && ! command -v docker &> /dev/null; then
        echo -e "${RED}❌ Docker/Docker Compose no está instalado${NC}"
        exit 1
    fi
    
    # Verificar archivo .env
    if [ ! -f "$(dirname "$0")/../.env" ]; then
        echo -e "${YELLOW}⚠️  Archivo .env no encontrado${NC}"
        echo "Creando .env de ejemplo..."
        cat > "$(dirname "$0")/../.env" << 'EOF'
# PostgreSQL Database Configuration
DATABASE_USER=admin
DATABASE_PASSWORD=admin123
DATABASE_NAME=flowchem
DATABASE_HOST=db
DATABASE_PORT=5432
DATABASE_URL=postgresql://admin:admin123@db:5432/flowchem

# Flow API Configuration
PORT=3000
HOST=0.0.0.0
RUST_LOG=info,flow_api=debug

# Python Configuration (for RDKit)
PYO3_PYTHON=/opt/conda/bin/python
PYTHON_SYS_EXECUTABLE=/opt/conda/bin/python
LD_LIBRARY_PATH=/opt/conda/lib
EOF
        echo -e "${GREEN}✅ Archivo .env creado${NC}"
    fi
    
    cd "$(dirname "$0")/.."
    
    echo -e "${YELLOW}🔍 Deteniendo servicios existentes...${NC}"
    docker-compose down || true
    
    if [ "$BUILD_MODE" = true ]; then
        echo -e "${YELLOW}🔨 Compilando imágenes Docker...${NC}"
        docker-compose build flow-api
    fi
    
    echo -e "${GREEN}🚀 Iniciando servicios (db + flow-api)...${NC}"
    docker-compose up -d db flow-api
    
    echo ""
    echo -e "${GREEN}✅ Servicios iniciados${NC}"
    echo -e "${GREEN}🌐 API:${NC} http://localhost:3000"
    echo -e "${GREEN}📚 Swagger UI:${NC} http://localhost:3000/docs"
    echo -e "${GREEN}📄 OpenAPI JSON:${NC} http://localhost:3000/api-doc/openapi.json"
    echo -e "${GREEN}❤️  Health Check:${NC} http://localhost:3000/health"
    echo -e "${GREEN}🗄️  PostgreSQL:${NC} localhost:5432"
    echo ""
    echo -e "${BLUE}📋 Ver logs:${NC} docker-compose logs -f flow-api"
    echo -e "${BLUE}🛑 Detener:${NC} docker-compose down"
    echo ""
    
    # Esperar a que el servicio esté listo
    echo -e "${YELLOW}⏳ Compilando y esperando a que la API esté lista (esto puede tomar 1-2 minutos en la primera ejecución)...${NC}"
    echo -e "${BLUE}💡 Tip: Puedes ver los logs de compilación con: docker-compose logs -f flow-api${NC}"
    echo ""
    
    for i in {1..120}; do
        if curl -s http://localhost:3000/health > /dev/null 2>&1; then
            echo ""
            echo -e "${GREEN}✅ API está lista y respondiendo${NC}"
            echo ""
            echo -e "${GREEN}🎉 Puedes acceder a:${NC}"
            echo -e "   ${BLUE}Swagger UI:${NC} http://localhost:3000/docs"
            echo -e "   ${BLUE}API Endpoints:${NC} http://localhost:3000/api/flows/cadma"
            break
        fi
        
        # Mostrar progreso cada 10 segundos
        if [ $((i % 10)) -eq 0 ]; then
            echo -e "${YELLOW}Esperando... (${i}s)${NC}"
        fi
        
        sleep 1
    done
    
    # Verificar si el servicio nunca respondió
    if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
        echo ""
        echo -e "${RED}❌ La API no respondió después de 2 minutos${NC}"
        echo -e "${YELLOW}Revisa los logs con: docker-compose logs flow-api${NC}"
        exit 1
    fi
    
    exit 0
fi

# Modo local (sin Docker)
echo -e "${BLUE}💻 Ejecutando en modo local${NC}"

# Verificar que existe la variable DATABASE_URL
if [ -z "$DATABASE_URL" ]; then
    echo -e "${YELLOW}⚠️  DATABASE_URL no está configurado${NC}"
    echo "Usando SQLite por defecto para desarrollo..."
    export DATABASE_URL="sqlite://./flow_api.db"
fi

echo -e "${GREEN}📊 Base de datos:${NC} $DATABASE_URL"

# Verificar si PostgreSQL está disponible (solo si se usa PostgreSQL)
if [[ $DATABASE_URL == postgresql://* ]] || [[ $DATABASE_URL == postgres://* ]]; then
    echo -e "${YELLOW}🔍 Verificando conexión a PostgreSQL...${NC}"
    
    # Extraer información de la URL
    DB_HOST=$(echo $DATABASE_URL | sed -E 's|.*://[^:]+:[^@]+@([^:/]+).*|\1|')
    DB_PORT=$(echo $DATABASE_URL | sed -E 's|.*://[^:]+:[^@]+@[^:]+:([0-9]+).*|\1|')
    DB_PORT=${DB_PORT:-5432}
    
    # Verificar conectividad
    if command -v pg_isready &> /dev/null; then
        if ! pg_isready -h "$DB_HOST" -p "$DB_PORT" &> /dev/null; then
            echo -e "${RED}❌ PostgreSQL no está disponible en $DB_HOST:$DB_PORT${NC}"
            echo "Para usar PostgreSQL, ejecuta:"
            echo "  docker-compose up -d db"
            echo "O usa el modo Docker con: $0 --docker"
            exit 1
        else
            echo -e "${GREEN}✅ PostgreSQL está disponible${NC}"
        fi
    else
        echo -e "${YELLOW}⚠️  pg_isready no está instalado, omitiendo verificación${NC}"
    fi
fi

# Cambiar al directorio raíz del proyecto
cd "$(dirname "$0")/.."

# Compilar si se especifica --build
if [ "$BUILD_MODE" = true ]; then
    echo -e "${YELLOW}🔨 Compilando flow-api...${NC}"
    cargo build --release --manifest-path crates/flow-api/Cargo.toml
fi

# Ejecutar migraciones
echo -e "${YELLOW}📦 Ejecutando migraciones de base de datos...${NC}"
if [[ $DATABASE_URL == sqlite://* ]]; then
    # Para SQLite, asegurar que el directorio existe
    DB_FILE=$(echo $DATABASE_URL | sed 's|sqlite://||')
    mkdir -p "$(dirname "$DB_FILE")"
    
    # Instalar diesel CLI si no está disponible
    if ! command -v diesel &> /dev/null; then
        echo "Instalando diesel CLI..."
        cargo install diesel_cli --no-default-features --features sqlite
    fi
    
    diesel migration run --database-url "$DATABASE_URL" --migration-dir crates/chem-persistence/migrations || true
elif [[ $DATABASE_URL == postgresql://* ]] || [[ $DATABASE_URL == postgres://* ]]; then
    # Instalar diesel CLI si no está disponible
    if ! command -v diesel &> /dev/null; then
        echo "Instalando diesel CLI..."
        cargo install diesel_cli --no-default-features --features postgres
    fi
    
    diesel migration run --database-url "$DATABASE_URL" --migration-dir crates/chem-persistence/migrations || true
fi

echo -e "${GREEN}✅ Migraciones completadas${NC}"

# Configurar variables de entorno adicionales
export PORT=${PORT:-3000}
export HOST=${HOST:-0.0.0.0}
export RUST_LOG=${RUST_LOG:-info,flow_api=debug}

echo ""
echo -e "${GREEN}🌐 Servidor:${NC} http://$HOST:$PORT"
echo -e "${GREEN}📚 Swagger UI:${NC} http://localhost:$PORT/docs"
echo -e "${GREEN}📄 OpenAPI JSON:${NC} http://localhost:$PORT/api-doc/openapi.json"
echo -e "${GREEN}❤️  Health Check:${NC} http://localhost:$PORT/health"
echo ""
echo -e "${YELLOW}Presiona Ctrl+C para detener el servidor${NC}"
echo ""

# Ejecutar la API
cd crates/flow-api
cargo run --release
