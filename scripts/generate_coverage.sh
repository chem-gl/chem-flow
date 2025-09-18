#!/usr/bin/env bash
set -euo pipefail

# scripts/generate_coverage.sh
# Construye la imagen Docker del proyecto (si no existe) y ejecuta los
# comandos necesarios para generar reportes de cobertura:
# - coverage/lcov.info
# - coverage/cobertura.xml (Cobertura XML)
# Requiere Docker instalado. Diseñado para ejecutarse desde la raíz del repo.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

IMAGE_NAME="flow-chem-ci"
FEATURES_ARG="" # Cambia si quieres compilar con features, e.g. "pg_demo"

echo "[coverage] Construyendo imagen Docker '${IMAGE_NAME}'..."
# Construimos la etapa 'base' (builder) que contiene rust/cargo para poder
# ejecutar cargo-tarpaulin dentro del contenedor. El Dockerfile define la
# primera etapa como `AS base`.
DEV_IMAGE_NAME="${IMAGE_NAME}-dev"
docker build -t ${DEV_IMAGE_NAME} --target base --build-arg FEATURES="${FEATURES_ARG}" -f Dockerfile .

# Opcional: construir la imagen runtime completa. Esto dispara la etapa
# `builder` que ejecuta `cargo build`. Por defecto lo dejamos desactivado
# para no forzar compilaciones pesadas durante generación de coverage.
# Para forzarlo exporta BUILD_RUNTIME=1 en el entorno.
if [ "${BUILD_RUNTIME:-0}" = "1" ]; then
  echo "[coverage] construyendo imagen runtime '${IMAGE_NAME}' (BUILD_RUNTIME=1)..."
  docker build -t ${IMAGE_NAME} --build-arg FEATURES="${FEATURES_ARG}" -f Dockerfile . || true
else
  echo "[coverage] salto construcción runtime (para forzar exporta BUILD_RUNTIME=1)"
fi


echo "[coverage] Ejecutando contenedor para generar coverage..."
mkdir -p coverage

# HOST cache directory (opcional). Si se exporta HOST_CACHE_DIR el script
# montará directorios desde el runner/host en lugar de usar volúmenes Docker.
# Esto es útil en CI (GitHub Actions) donde restauramos caches en
# `${GITHUB_WORKSPACE}/.cache/flow` y los montamos en el contenedor para
# acelerar builds.
HOST_CACHE_DIR="${HOST_CACHE_DIR:-}"

# --- Caching: crear volúmenes Docker para acelerar builds locales ---
# Estos volúmenes guardan: target (artefactos compilación), registry/git (dependencias),
# conda pkgs y pip cache. Útil para ejecuciones locales repetidas. En runners
# CI hospedados (GitHub Actions) los runners son efímeros y estos volúmenes no
# persistirán entre ejecuciones.
VOLUMES=()

if [ -n "$HOST_CACHE_DIR" ]; then
  echo "[coverage] Usando host cache en: $HOST_CACHE_DIR"
  mkdir -p "$HOST_CACHE_DIR/target" "$HOST_CACHE_DIR/cargo/registry" "$HOST_CACHE_DIR/cargo/git" "$HOST_CACHE_DIR/conda_pkgs" "$HOST_CACHE_DIR/pip"
  VOLUMES+=("-v ${HOST_CACHE_DIR}/target:/workspace/target")
  VOLUMES+=("-v ${HOST_CACHE_DIR}/cargo/registry:/root/.cargo/registry")
  VOLUMES+=("-v ${HOST_CACHE_DIR}/cargo/git:/root/.cargo/git")
  VOLUMES+=("-v ${HOST_CACHE_DIR}/conda_pkgs:/opt/conda/pkgs")
  VOLUMES+=("-v ${HOST_CACHE_DIR}/pip:/root/.cache/pip")
else
  V_TARGET="flow_cargo_target"
  V_REGISTRY="flow_cargo_registry"
  V_GIT="flow_cargo_git"
  V_CONDA_PKGS="flow_conda_pkgs"
  V_PIP_CACHE="flow_pip_cache"

  for v in "$V_TARGET" "$V_REGISTRY" "$V_GIT" "$V_CONDA_PKGS" "$V_PIP_CACHE"; do
    if ! docker volume inspect "$v" >/dev/null 2>&1; then
      echo "[coverage] creando volumen docker: $v"
      docker volume create "$v" >/dev/null
    fi
  done

  # Montajes de volumen que se pasarán al docker run
  VOLUMES=(
    "-v ${V_TARGET}:/workspace/target"
    "-v ${V_REGISTRY}:/root/.cargo/registry"
    "-v ${V_GIT}:/root/.cargo/git"
    "-v ${V_CONDA_PKGS}:/opt/conda/pkgs"
    "-v ${V_PIP_CACHE}:/root/.cache/pip"
  )
fi


# Nombre único para el contenedor (para limpieza en trap)
CN="flow-coverage-$$-$(date +%s)"

cleanup() {
  echo "[coverage] Ejecutando cleanup..."
  # intentar eliminar el contenedor si sigue existiendo (ignorar errores)
  docker rm -f "$CN" 2>/dev/null || true
}
trap cleanup EXIT

# Construir argumentos de volúmenes para pasar a docker run
VOLUMES_ARGS=""
for v in "${VOLUMES[@]}"; do
  VOLUMES_ARGS+="$v "
done

docker run --name "$CN" --rm --cap-add=SYS_PTRACE --security-opt seccomp=unconfined \
  ${VOLUMES_ARGS} \
  -e PYO3_PYTHON=/opt/conda/bin/python \
  -e PYTHON_SYS_EXECUTABLE=/opt/conda/bin/python \
  -e LD_LIBRARY_PATH=/opt/conda/lib \
  -v "${ROOT_DIR}":/workspace -w /workspace ${DEV_IMAGE_NAME} \
  bash -lc 'set -euo pipefail
    echo "[coverage/container] asegurando cargo-tarpaulin (solo instalar si falta)..."
    if ! command -v cargo-tarpaulin >/dev/null 2>&1; then
      echo "[coverage/container] cargo-tarpaulin no encontrado; instalando..."
      cargo install cargo-tarpaulin || true
    else
      echo "[coverage/container] cargo-tarpaulin ya instalado; salto instalación"
    fi

    echo "[coverage/container] generando LCOV y Cobertura XML en una sola ejecución (y comprobando umbral)..."
    # Ejecutar tarpaulin una vez produciendo ambos formatos para evitar recompilaciones dobles
    cargo tarpaulin --workspace --out Lcov --out Xml --output-dir coverage --fail-under 90

    echo "[coverage/container] listado de coverage/"
    ls -la coverage || true
  echo "[coverage/container] intentando convertir cobertura a Sonar generic format (coverage/sonar-generic-coverage.xml)"
  # Convertir cobertura.xml (tarpaulin produces Cobertura-like XML) a Sonar Generic Coverage XML
  if [ -f coverage/cobertura.xml ]; then
    python3 - <<'PY'
import os,sys,xml.etree.ElementTree as ET
cov_in='coverage/cobertura.xml'
out='coverage/sonar-generic-coverage.xml'
if not os.path.exists(cov_in):
  print('no cobertura found', file=sys.stderr); sys.exit(2)
try:
  tree = ET.parse(cov_in)
  root = tree.getroot()
except Exception as e:
  print('parse error', e, file=sys.stderr); sys.exit(3)

cov = ET.Element('coverage', {'version':'1'})

def collect_lines(root):
  lines_map = {}
  # Find all <class> elements and their lines
  for class_el in root.findall('.//class'):
    filename = class_el.get('filename') or class_el.get('name')
    if not filename:
      continue
    for lines_el in class_el.findall('.//lines'):
      for line in lines_el.findall('line'):
        num = line.get('number')
        hits = line.get('hits') or line.get('count') or line.get('hit')
        try:
          covered = 'true' if int(hits or '0')>0 else 'false'
        except Exception:
          covered = 'false'
        lines_map.setdefault(filename, {})[int(num)] = covered
  return lines_map

lines_map = collect_lines(root)
if not lines_map:
  # Try alternative shapes: some tarpaulin outputs use <lines><line number="..." hits="..."/></lines>
  print('no lines found in cobertura; skipping conversion', file=sys.stderr)
else:
  for fname, lines in lines_map.items():
    file_el = ET.SubElement(cov, 'file', {'path': fname})
    for ln, covered in sorted(lines.items()):
      ET.SubElement(file_el, 'lineToCover', {'lineNumber': str(ln), 'covered': covered})
  ET.ElementTree(cov).write(out, encoding='utf-8', xml_declaration=True)
  print(out)
PY
    echo "[coverage/container] listado de coverage/ tras conversión"
    ls -la coverage || true
  else
    echo "[coverage/container] coverage/cobertura.xml no encontrado; no se generó sonar-generic-coverage.xml" || true
  fi
  '

# Al salir del docker run, el trap cleanup intentará eliminar el contenedor
echo "[coverage] Artefactos generados en: ${ROOT_DIR}/coverage"
ls -la coverage || true

echo "[coverage] Hecho. Revisa coverage/lcov.info y coverage/*.xml"

# Si existe lcov.info, convertir a HTML usando un contenedor ligero con lcov/genhtml
if [ -f coverage/lcov.info ]; then
  echo "[coverage] Convirtiendo coverage/lcov.info a HTML (coverage/html) usando contenedor temporal)..."
  # Usamos ubuntu:22.04 y instalamos lcov
  docker run --rm -v "${ROOT_DIR}/coverage":/coverage -w /coverage ubuntu:22.04 bash -lc '
    apt-get update && apt-get install -y lcov gettext-base --no-install-recommends && \
    genhtml -o html lcov.info || true'
  echo "[coverage] HTML generado en coverage/html"
  ls -la coverage/html || true
else
  echo "[coverage] coverage/lcov.info no encontrado; salto conversión a HTML"
fi

exit 0
