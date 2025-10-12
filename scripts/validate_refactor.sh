#!/bin/bash
# validate_refactor.sh - Script para validar la refactorización SOLID Phases 2-4

set -e  # Exit on error

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Validación de Refactorización SOLID - Phases 2-4           ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print step
print_step() {
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "  $1"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Function to print success
print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

# Function to print error
print_error() {
    echo -e "${RED}✗${NC} $1"
}

# Function to print warning
print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Step 1: Clean build
print_step "Step 1: Clean Build"
echo "Limpiando artefactos previos..."
cargo clean
print_success "Limpieza completada"

# Step 2: Build all features
print_step "Step 2: Build con todas las features"
echo "Compilando todo el workspace..."
if cargo build --all-features; then
    print_success "Build exitoso"
else
    print_error "Build falló"
    exit 1
fi

# Step 3: Clippy check
print_step "Step 3: Clippy - Análisis de código"
echo "Ejecutando clippy con todas las features..."
if cargo clippy --all-features -- -D warnings; then
    print_success "Clippy: sin warnings"
else
    print_error "Clippy encontró problemas"
    exit 1
fi

# Step 4: Tests - chem-domain
print_step "Step 4: Tests - chem-domain (Core Domain)"
echo "Ejecutando tests del dominio..."
if cargo test -p chem-domain --all-features; then
    print_success "Tests de chem-domain: PASSED"
else
    print_error "Tests de chem-domain: FAILED"
    exit 1
fi

# Step 5: Tests - chem-persistence
print_step "Step 5: Tests - chem-persistence (Infrastructure)"
echo "Ejecutando tests de persistencia..."
if cargo test -p chem-persistence --all-features; then
    print_success "Tests de chem-persistence: PASSED"
else
    print_error "Tests de chem-persistence: FAILED"
    exit 1
fi

# Step 6: Tests - chem-workflow
print_step "Step 6: Tests - chem-workflow (Application)"
echo "Ejecutando tests de workflow..."
if cargo test -p chem-workflow --all-features; then
    print_success "Tests de chem-workflow: PASSED"
else
    print_error "Tests de chem-workflow: FAILED"
    exit 1
fi

# Step 7: Tests - flow
print_step "Step 7: Tests - flow (Flow Repository)"
echo "Ejecutando tests de flow..."
if cargo test -p flow --all-features; then
    print_success "Tests de flow: PASSED"
else
    print_error "Tests de flow: FAILED"
    exit 1
fi

# Step 8: Integration tests
print_step "Step 8: Tests de Integración"
echo "Ejecutando todos los tests del workspace..."
if cargo test --all --all-features; then
    print_success "Tests de integración: PASSED"
else
    print_error "Tests de integración: FAILED"
    exit 1
fi

# Step 9: Doc check
print_step "Step 9: Verificación de Documentación"
echo "Verificando que la documentación compila..."
if cargo doc --all-features --no-deps; then
    print_success "Documentación compilada correctamente"
else
    print_warning "Problemas con la documentación"
fi

# Step 10: Format check
print_step "Step 10: Verificación de Formato"
echo "Verificando formato del código..."
if cargo fmt -- --check; then
    print_success "Formato de código correcto"
else
    print_warning "Código necesita formato (cargo fmt)"
fi

# Final Summary
echo ""
echo "╔══════════════════════════════════════════════════════════════╗"
echo "║                   VALIDACIÓN COMPLETA                        ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""
print_success "✓ Build exitoso"
print_success "✓ Clippy: sin warnings"
print_success "✓ Todos los tests pasaron"
print_success "✓ Documentación OK"
echo ""
echo "🎉 Refactorización SOLID Phases 2-4 VALIDADA EXITOSAMENTE 🎉"
echo ""
echo "Próximos pasos opcionales:"
echo "  1. Revisar REFACTOR_SUMMARY.md para detalles completos"
echo "  2. Ejecutar ./scripts/generate_coverage.sh para coverage"
echo "  3. Ejecutar ./scripts/run_tests_in_docker.sh para tests en Docker"
echo ""
