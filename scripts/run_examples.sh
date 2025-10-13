#!/usr/bin/env bash
set -euo pipefail
# run_examples.sh
# Arranca Postgres y app-dev, espera salud, luego ejecuta los ejemplos
# (example-domain, example-main y persistence_simple_usage) usando
# Postgres + RDKit (Python) dentro del contenedor app-dev.
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "Iniciando servicios docker-compose (db, app-dev) para ejemplos..."
docker-compose up -d db app-dev

DB_CONTAINER=$(docker-compose ps -q db)
if [ -z "$DB_CONTAINER" ]; then
  echo "No se encontró el contenedor de la base de datos (db)." >&2
  exit 1
fi

# Esperar a que Postgres esté healthy
for i in $(seq 1 60); do
  STATUS=$(docker inspect --format='{{.State.Health.Status}}' "$DB_CONTAINER" 2>/dev/null || echo "unknown")
  if [ "$STATUS" = "healthy" ]; then
    break
  fi
  echo "Esperando a db (estado: $STATUS)..."
  sleep 2
done

APP_DEV_CONTAINER=$(docker-compose ps -q app-dev)
if [ -z "$APP_DEV_CONTAINER" ]; then
  echo "No se encontró el contenedor app-dev." >&2
  exit 1
fi

# Variables para Postgres (usa .env si está disponible)
DB_USER=${DATABASE_USER:-admin}
DB_PASS=${DATABASE_PASS:-admin123}
DB_NAME=${DATABASE_NAME:-mydatabase}
# Dentro del contenedor app-dev, el host de Postgres es el nombre del servicio Docker: "db"
DB_HOST=db
DB_PORT=5432
PG_URL="postgres://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}"

echo
echo "Selecciona qué ejemplo quieres ejecutar:"
echo "  1) example-domain (dominio)"
echo "  2) example-main (flow CLI)"
echo "  3) chem-persistence/persistence_simple_usage"
echo "  4) cadma_example (chem-workflow)"
echo "  5) Todos"
read -r -p "Opción [1-4]: " EXAMPLE_CHOICE
echo
echo "Ejecutando ejemplos con Postgres+RDKit dentro del contenedor app-dev..."
# Exportar CHEM_DB_URL para los helpers de chem-persistence/chem-domain
docker-compose exec \
  -e DATABASE_URL="$PG_URL" \
  -e CHEM_DB_URL="$PG_URL" \
  -e EXAMPLE_CHOICE="$EXAMPLE_CHOICE" \
  -e PYO3_PYTHON="/opt/conda/bin/python" \
  -e PYTHON_SYS_EXECUTABLE="/opt/conda/bin/python" \
  app-dev bash -lc '
  set -euo pipefail
  cd /workspace
  echo "DATABASE_URL=$DATABASE_URL"
  echo "CHEM_DB_URL=$CHEM_DB_URL"
  cargo --version
  case "${EXAMPLE_CHOICE}" in
    1)
      echo "Ejecutando example-domain..."
      cargo run --example example-domain --features="integration_examples postgres pg_demo"
      ;;
    2)
      echo "Ejecutando example-main..."
      cargo run --example example-main --features="integration_examples postgres pg_demo"
      ;;
    3)
      echo "Ejecutando chem-persistence/persistence_simple_usage..."
      (cd crates/chem-persistence && cargo run --example persistence_simple_usage --features postgres)
      ;;
    4)
      echo "Ejecutando chem-workflow/cadma_example..."
      cargo run -p chem-workflow --example cadma_example --features="integration_examples pg postgres"
      ;;
    5)
      echo "Ejecutando example-domain..."
      cargo run --example example-domain --features="integration_examples postgres pg_demo"
      echo "Ejecutando example-main..."
      cargo run --example example-main --features="integration_examples postgres pg_demo"
      echo "Ejecutando chem-persistence/persistence_simple_usage..."
      (cd crates/chem-persistence && cargo run --example persistence_simple_usage --features postgres)
      echo "Ejecutando chem-workflow/cadma_example..."
      cargo run -p chem-workflow --example cadma_example --features="integration_examples pg postgres"
      ;;
    *)
      echo "Opción inválida: ${EXAMPLE_CHOICE}" >&2
      exit 2
      ;;
  esac
'

EXIT_CODE=$?
if [ $EXIT_CODE -ne 0 ]; then
  echo "Alguno de los ejemplos falló (exit $EXIT_CODE)." >&2
else
  echo "Ejemplos ejecutados correctamente (Postgres + RDKit)."
fi
exit $EXIT_CODE
