#!/usr/bin/env bash
# run_tests_with_mocks.sh
# Script para ejecutar pruebas con mocks (sin necesidad de RDKit)
set -e
# Colores para output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color
echo -e "${YELLOW}Formateando el código (cargo fmt) y ejecutando tests con mock_rdkit...${NC}"
echo -e "${YELLOW}Esto permite ejecutar pruebas sin tener RDKit instalado.${NC}"
# Formatear antes de ejecutar tests (no es obligatorio, pero ayuda a evitar fallos por estilo)
cargo fmt --all || echo -e "${YELLOW}cargo fmt falló o no está disponible; continuando...${NC}"

# Ejecutar los tests con mock_rdkit
RUST_BACKTRACE=1 cargo test --workspace --all-targets --features mock_rdkit
if [ $? -eq 0 ]; then
	echo -e "${GREEN}Tests con mocks completados exitosamente${NC}"
else
	echo -e "${RED}Tests con mocks FALLARON${NC}"
	exit 1
fi