#!/bin/bash

# Script para ejecutar la API RESTful de CADMA
# Uso: ./scripts/run_flow_api.sh [--build] [--docker]

set -e

# Ensure cargo binaries are on PATH so `diesel` installed below is immediately available
export PATH="$HOME/.cargo/bin:$PATH"

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
# Para despliegues por defecto usamos PostgreSQL apuntando al servicio `db`.
if [ -z "$DATABASE_URL" ]; then
    echo -e "${YELLOW}⚠️  DATABASE_URL no está configurado. Usando PostgreSQL por defecto (servicio 'db').${NC}"
    # String por defecto coherente con el .env que creamos más arriba
    export DATABASE_URL="postgresql://admin:admin123@db:5432/flowchem"
fi

echo -e "${GREEN}📊 Base de datos:${NC} $DATABASE_URL"

# Verificar si PostgreSQL está disponible (solo si se usa PostgreSQL)
if [[ $DATABASE_URL == postgresql://* ]] || [[ $DATABASE_URL == postgres://* ]]; then
    # If running locally (not in Docker mode) and DATABASE_URL points to the
    # service name 'db', rewrite it to 'localhost' so the host process can
    # connect to the Postgres container via the published port (5432).
    if [ "$DOCKER_MODE" != true ] && echo "$DATABASE_URL" | grep -q '@db:'; then
        echo -e "${YELLOW}⚠️ Ejecutando localmente pero DATABASE_URL apunta a 'db' — usando 'localhost' para conexiones locales.${NC}"
        DATABASE_URL=$(echo "$DATABASE_URL" | sed 's/@db:/@localhost:/')
        export DATABASE_URL
        echo -e "${GREEN}📊 Base de datos actualizada a:${NC} $DATABASE_URL"
    fi
    echo -e "${YELLOW}🔍 Verificando conexión a PostgreSQL...${NC}"
    
    # Extraer información de la URL
    DB_HOST=$(echo $DATABASE_URL | sed -E 's|.*://[^:]+:[^@]+@([^:/]+).*|\1|')
    DB_PORT=$(echo $DATABASE_URL | sed -E 's|.*://[^:]+:[^@]+@[^:]+:([0-9]+).*|\1|')
    DB_PORT=${DB_PORT:-5432}
    
    # Verificar conectividad
    if command -v pg_isready &> /dev/null; then
        if ! pg_isready -h "$DB_HOST" -p "$DB_PORT" &> /dev/null; then
            # If pg isn't reachable, try to start db with docker-compose (if available)
            # so local users don't have to run it manually.
            if command -v docker-compose &> /dev/null || (command -v docker &> /dev/null && docker compose version >/dev/null 2>&1); then
                # choose compose command
                if command -v docker-compose &> /dev/null; then
                    COMPOSE_CMD="docker-compose"
                else
                    COMPOSE_CMD="docker compose"
                fi
                echo -e "${YELLOW}⚠️ PostgreSQL no responde en $DB_HOST:$DB_PORT. Intentando levantar servicio 'db' con Docker Compose...${NC}"
                $COMPOSE_CMD up -d db

                # wait for container healthy (use docker inspect if possible)
                DB_CONTAINER=$($COMPOSE_CMD ps -q db 2>/dev/null || true)
                if [[ -n "$DB_CONTAINER" ]]; then
                    echo "Esperando a que el contenedor db esté healthy..."
                    for i in {1..60}; do
                        STATUS=$(docker inspect --format='{{json .State.Health.Status}}' "$DB_CONTAINER" 2>/dev/null || echo "null")
                        if [[ "$STATUS" == '"healthy"' ]]; then
                            echo -e "${GREEN}✅ Postgres container is healthy${NC}"
                            break
                        fi
                        if [[ $i -eq 60 ]]; then
                            echo -e "${RED}❌ Timeout esperando el healthcheck del contenedor db${NC}"
                            break
                        fi
                        sleep 2
                    done
                else
                    # fallback: wait for pg_isready on localhost:5432
                    echo "Esperando a que Postgres acepte conexiones en $DB_HOST:$DB_PORT..."
                    for i in {1..60}; do
                        if pg_isready -h "$DB_HOST" -p "$DB_PORT" &> /dev/null; then
                            echo -e "${GREEN}✅ PostgreSQL está disponible${NC}"
                            break
                        fi
                        sleep 1
                    done
                fi
                # final check
                if ! pg_isready -h "$DB_HOST" -p "$DB_PORT" &> /dev/null; then
                    echo -e "${RED}❌ PostgreSQL no está disponible en $DB_HOST:$DB_PORT tras intentar levantarlo con Docker Compose${NC}"
                    exit 1
                fi
            else
                echo -e "${RED}❌ PostgreSQL no está disponible en $DB_HOST:$DB_PORT${NC}"
                echo "Para usar PostgreSQL, ejecuta:"
                echo "  docker-compose up -d db"
                echo "O usa el modo Docker con: $0 --docker"
                exit 1
            fi
        else
            echo -e "${GREEN}✅ PostgreSQL está disponible${NC}"
        fi
    else
        echo -e "${YELLOW}⚠️  pg_isready no está instalado, omitiendo verificación${NC}"
        # If pg_isready is missing, but docker-compose is available, try to bring up db so the user doesn't need to.
        if command -v docker-compose &> /dev/null || (command -v docker &> /dev/null && docker compose version >/dev/null 2>&1); then
            if command -v docker-compose &> /dev/null; then
                COMPOSE_CMD="docker-compose"
            else
                COMPOSE_CMD="docker compose"
            fi
            echo -e "${YELLOW}ℹ️ pg_isready no está instalado: intentando levantar 'db' con Docker Compose para comodidad...${NC}"
            $COMPOSE_CMD up -d db
            DB_CONTAINER=$($COMPOSE_CMD ps -q db 2>/dev/null || true)
            if [[ -n "$DB_CONTAINER" ]]; then
                echo "Esperando a que el contenedor db esté healthy..."
                for i in {1..60}; do
                    STATUS=$(docker inspect --format='{{json .State.Health.Status}}' "$DB_CONTAINER" 2>/dev/null || echo "null")
                    if [[ "$STATUS" == '"healthy"' ]]; then
                        echo -e "${GREEN}✅ Postgres container is healthy${NC}"
                        break
                    fi
                    sleep 2
                done
            fi
        fi
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
        cargo install diesel_cli --no-default-features --features sqlite --force
    fi
    
    diesel migration run --database-url "$DATABASE_URL" --migration-dir crates/chem-persistence/migrations || true
elif [[ $DATABASE_URL == postgresql://* ]] || [[ $DATABASE_URL == postgres://* ]]; then
    # Instalar diesel CLI si no está disponible
    if ! command -v diesel &> /dev/null; then
        echo "Instalando diesel CLI..."
        cargo install diesel_cli --no-default-features --features postgres --force
    fi

    # Ensure the target database exists. Parse DATABASE_URL for connection parts.
    proto_removed="${DATABASE_URL#*://}"
    userpass="${proto_removed%%@*}"
    DB_USER="${userpass%%:*}"
    DB_PASS="${userpass#*:}"
    hostportdb="${proto_removed#*@}"
    hostport="${hostportdb%%/*}"
    DB_HOST_PARSED="${hostport%%:*}"
    DB_PORT_PARSED="${hostport#*:}"
    DB_NAME_PARSED="${hostportdb#*/}"
    DB_NAME_PARSED="${DB_NAME_PARSED%%\?*}"

    # prefer docker-compose to create DB if available
    if command -v docker-compose &> /dev/null || (command -v docker &> /dev/null && docker compose version >/dev/null 2>&1); then
        if command -v docker-compose &> /dev/null; then
            COMPOSE_CMD="docker-compose"
        else
            COMPOSE_CMD="docker compose"
        fi
        DB_CONTAINER=$($COMPOSE_CMD ps -q db 2>/dev/null || true)
        if [[ -n "$DB_CONTAINER" ]]; then
            echo "Comprobando si la base de datos '$DB_NAME_PARSED' existe dentro del contenedor db..."
            EXISTS=$(docker exec "$DB_CONTAINER" psql -U "$DB_USER" -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname='$DB_NAME_PARSED'" 2>/dev/null || true)
            if [[ "$EXISTS" != "1" ]]; then
                echo "Base de datos '$DB_NAME_PARSED' no encontrada. Creando dentro del contenedor db..."
                docker exec "$DB_CONTAINER" psql -U "$DB_USER" -d postgres -c "CREATE DATABASE \"$DB_NAME_PARSED\";" || true
            else
                echo "Base de datos '$DB_NAME_PARSED' ya existe.";
            fi
        else
            echo "No se detectó contenedor 'db' para creación automática. Intentando crear con psql local..."
            if command -v psql &> /dev/null; then
                export PGPASSWORD="$DB_PASS"
                EXISTS=$(psql -h "$DB_HOST_PARSED" -U "$DB_USER" -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname='$DB_NAME_PARSED'" 2>/dev/null || true)
                if [[ "$EXISTS" != "1" ]]; then
                    echo "Base de datos '$DB_NAME_PARSED' no encontrada. Creando con psql local..."
                    psql -h "$DB_HOST_PARSED" -U "$DB_USER" -d postgres -c "CREATE DATABASE \"$DB_NAME_PARSED\";" || true
                else
                    echo "Base de datos '$DB_NAME_PARSED' ya existe.";
                fi
                unset PGPASSWORD
            else
                echo "No se detectó 'psql' local y no hay contenedor 'db' para crear la base de datos. Por favor crea la BD manualmente y luego reintenta."
            fi
        fi
    else
        # No docker-compose and no docker compose; fallback to local psql if available
        if command -v psql &> /dev/null; then
            export PGPASSWORD="$DB_PASS"
            EXISTS=$(psql -h "$DB_HOST_PARSED" -U "$DB_USER" -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname='$DB_NAME_PARSED'" 2>/dev/null || true)
            if [[ "$EXISTS" != "1" ]]; then
                echo "Base de datos '$DB_NAME_PARSED' no encontrada. Creando con psql local..."
                psql -h "$DB_HOST_PARSED" -U "$DB_USER" -d postgres -c "CREATE DATABASE \"$DB_NAME_PARSED\";" || true
            else
                echo "Base de datos '$DB_NAME_PARSED' ya existe.";
            fi
            unset PGPASSWORD
        else
            echo "No hay docker-compose ni psql disponible para crear la base de datos. Por favor crea '$DB_NAME_PARSED' manualmente y reintenta."
        fi
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
# If using Postgres, ensure the crate is built with the 'pg' feature so
# chem-persistence is compiled with Postgres support.
if [[ $DATABASE_URL == postgresql://* ]] || [[ $DATABASE_URL == postgres://* ]]; then
    echo -e "${YELLOW}🔨 Compilando flow-api con soporte Postgres (feature 'pg')...${NC}"
    cargo build --release --features pg || {
        echo -e "${RED}❌ Falló la compilación con feature 'pg'. Intenta ejecutar:\n  cargo build --release --features pg${NC}";
        exit 1
    }
    echo -e "${GREEN}✅ Compilación con 'pg' completa${NC}"
    echo -e "${YELLOW}▶ Ejecutando flow-api (con 'pg')...${NC}"
    cargo run --release --features pg
else
    cargo run --release
fi
