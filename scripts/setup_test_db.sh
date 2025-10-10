#!/usr/bin/env bash
# setup_test_db.sh
# Script para configurar bases de datos de prueba

set -e

# Colores para output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Función para verificar si PostgreSQL está disponible
check_postgres() {
  echo -e "${YELLOW}Verificando disponibilidad de PostgreSQL...${NC}"
  if command -v psql &> /dev/null; then
    echo -e "${GREEN}PostgreSQL está instalado${NC}"
    return 0
  else
    echo -e "${RED}PostgreSQL no está disponible${NC}"
    return 1
  fi
}

# Función para crear base de datos de prueba PostgreSQL
setup_postgres_test_db() {
  echo -e "${YELLOW}Configurando base de datos de prueba en PostgreSQL...${NC}"
  
  # Generar nombre único para la base de datos de prueba
  TEST_DB="flow_chem_test_$(date +%s)"
  
  # Crear base de datos
  psql -c "CREATE DATABASE $TEST_DB;" postgres
  
  # Exportar variable de entorno con la URL de conexión
  export DATABASE_URL="postgres://postgres:postgres@localhost/$TEST_DB"
  echo -e "${GREEN}Base de datos PostgreSQL creada: $TEST_DB${NC}"
  echo -e "${YELLOW}Usar: export DATABASE_URL=\"$DATABASE_URL\"${NC}"
}

# Función para configurar SQLite
setup_sqlite_test_db() {
  echo -e "${YELLOW}Configurando base de datos de prueba en SQLite...${NC}"
  
  # Crear directorio temporal para la base de datos
  TEMP_DIR=$(mktemp -d)
  TEST_DB="$TEMP_DIR/flow_chem_test.db"
  
  # Exportar variable de entorno con la URL de conexión
  export DATABASE_URL="sqlite://$TEST_DB"
  echo -e "${GREEN}Base de datos SQLite creada: $TEST_DB${NC}"
  echo -e "${YELLOW}Usar: export DATABASE_URL=\"$DATABASE_URL\"${NC}"
}

# Menú principal
echo -e "${YELLOW}Seleccione el tipo de base de datos para pruebas:${NC}"
echo "1) PostgreSQL"
echo "2) SQLite"

read -p "Opción: " DB_OPTION

case $DB_OPTION in
  1)
    if check_postgres; then
      setup_postgres_test_db
    else
      echo -e "${RED}PostgreSQL no está disponible, usando SQLite.${NC}"
      setup_sqlite_test_db
    fi
    ;;
  2)
    setup_sqlite_test_db
    ;;
  *)
    echo -e "${RED}Opción inválida, usando SQLite.${NC}"
    setup_sqlite_test_db
    ;;
esac

echo -e "${GREEN}Configuración de base de datos de prueba completada.${NC}"