#!/usr/bin/env bash
set -euo pipefail
# run_tests_in_docker.sh
# Starts docker-compose (db + app-dev), waits for services to be healthy,
# then runs cargo test inside the app-dev container so RDKit/Python are available.
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"
# Iniciar servicios docker-compose (db, app-dev)
echo "Iniciando servicios docker-compose (db, app-dev)..."
docker-compose up -d db app-dev
echo "Esperando a que la base de datos esté saludable (healthcheck de docker-compose)..."
# Wait until db is healthy and app-dev container is running
sleep 3
# Wait for the db service to be healthy (pg_isready) using docker inspect
DB_CONTAINER=$(docker-compose ps -q db)
  if [[ -z "$DB_CONTAINER" ]]; then
  echo "contenedor db no encontrado" >&2
  docker-compose ps
  exit 1
fi
# Poll the health status
for i in {1..60}; do
  STATUS=$(docker inspect --format='{{json .State.Health.Status}}' "$DB_CONTAINER" 2>/dev/null || echo "null")
  if [[ "$STATUS" == '"healthy"' ]]; then
    echo "db is healthy"
    break
  fi
  echo "db status: $STATUS - waiting..."
  sleep 2
done
# Ensure app-dev container is running
APPDEV_CONTAINER=$(docker-compose ps -q app-dev)
  if [[ -z "$APPDEV_CONTAINER" ]]; then
  echo "contenedor app-dev no encontrado" >&2
  docker-compose ps
  exit 1
fi
# Copy local workspace into container if not mounted (the compose file mounts the workspace)
# Run tests inside the app-dev container. The base image includes conda/rdkit and rustup.
echo "Ejecutando tests de cargo dentro del contenedor app-dev..."
# Use exec to run command and stream logs to stdout
# Run only the chem-domain package tests (failing ones) to reduce runtime
docker exec -it "$APPDEV_CONTAINER" /bin/bash -lc "cd /workspace && export RUST_BACKTRACE=1 && cargo test -p chem-domain --verbose"
# Generate LCOV coverage inside the container (requires cargo-tarpaulin installed in the image)
echo "Generando cobertura (LCOV) dentro del contenedor app-dev..."
docker exec -it "$APPDEV_CONTAINER" /bin/bash -lc "cd /workspace && export RUST_BACKTRACE=1 && cargo tarpaulin --out Lcov || true"
# Ensure host coverage dir exists and copy lcov.info from container to host so Sonar can read it
mkdir -p "$REPO_ROOT/coverage"
CONTAINER_LCOV_PATH="/workspace/tarpaulin-lcov.info"
echo "Copiando LCOV desde el contenedor ($CONTAINER_LCOV_PATH) a host coverage/lcov.info"
docker cp "$APPDEV_CONTAINER":"$CONTAINER_LCOV_PATH" "$REPO_ROOT/coverage/lcov.info" || echo "No lcov.info produced or copy failed"
EXIT_CODE=$?
if [[ $EXIT_CODE -ne 0 ]]; then
  echo "Los tests fallaron (exit $EXIT_CODE). Revisa la salida anterior." >&2
else
  echo "Tests ejecutados correctamente dentro del contenedor."
fi
exit $EXIT_CODE
