#!/bin/bash
# Script para recrear la base de datos y aplicar migraciones

set -euo pipefail

# Cargar variables de entorno
if [ -f .env ]; then
  source .env
fi

DB_NAME="${DATABASE_NAME:-mydatabase}"
DB_USER="${DATABASE_USER:-admin}"
DB_HOST="${DATABASE_HOST:-localhost}"
DB_PORT="${DATABASE_PORT:-5432}"

echo "🔄 Recreando base de datos '$DB_NAME'..."

# Conectar a postgres (base de datos por defecto) para recrear la base de datos
PGPASSWORD="${DATABASE_PASSWORD:-admin123}" psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d postgres <<SQL
-- Terminar conexiones activas
SELECT pg_terminate_backend(pg_stat_activity.pid)
FROM pg_stat_activity
WHERE pg_stat_activity.datname = '$DB_NAME'
  AND pid <> pg_backend_pid();

-- Eliminar y recrear la base de datos
DROP DATABASE IF EXISTS $DB_NAME;
CREATE DATABASE $DB_NAME;
SQL

echo "✅ Base de datos recreada"
echo "📦 Aplicando migraciones con Diesel..."

# Las migraciones se aplicarán automáticamente al iniciar la aplicación
# o puedes ejecutar: cargo run --example cadma_example

echo "✅ Listo! La base de datos está limpia y lista para usar."
echo "   Las migraciones se aplicarán automáticamente al ejecutar la aplicación."
