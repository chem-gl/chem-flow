# Coverage helper

Este repo incluye un helper para generar coverage reproducible y rápido.

Opciones para ejecutar:

1. Docker Compose (recomendado para desarrollo local rápido)

- Levanta un servicio `coverage-volumes` que crea los volúmenes nombrados y los mantiene.
- Ejecuta `coverage-runner` que usa la etapa `base` (builder) con Rust y Python.

Comandos:

```bash
# Levantar volúmenes en background (crea/usa los volúmenes nombrados)
docker compose -f docker-compose.coverage.yml up -d coverage-volumes

# Ejecutar el runner (ejecuta el script y luego se detiene)
docker compose -f docker-compose.coverage.yml run --rm coverage-runner

# Parar y limpiar (opcional)
docker compose -f docker-compose.coverage.yml down
```

2. Script local (usa volúmenes Docker por defecto)

```bash
chmod +x scripts/generate_coverage.sh
./scripts/generate_coverage.sh
```

3. CI (GitHub Actions)

El workflow `.github/workflows/ci.yaml` usa `actions/cache` para restaurar caches
en `${{ github.workspace }}/.cache/flow` y pasa esa ruta al script como
`HOST_CACHE_DIR`, de forma que el contenedor monta el cache del runner y evita
volver a descargar dependencias en cada ejecución.

Notas:

- Si los volúmenes ocupan mucho espacio puedes limpiarlos con `docker volume rm`
  o `docker volume prune`.
- El umbral de cobertura por defecto está fijado en 90% (tarpaulin `--fail-under 90`).
