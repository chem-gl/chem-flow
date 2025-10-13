# flow-chem

flow-chem es una plataforma modular en Rust para modelar, versionar y persistir flujos de trabajo químicos (molecular families, propiedades y workflows). Este README explica cómo configurar, levantar y desarrollar el proyecto, además de describir su arquitectura y uso con diagramas.

## Contenido

- Visión general
- Requisitos
# flow-chem

flow-chem es una plataforma modular en Rust para modelar, versionar y persistir flujos de trabajo químicos (familias moleculares, propiedades y workflows). Este README explica cómo configurar, levantar y desarrollar el proyecto, además de describir su arquitectura y uso con diagramas.

## Contenido

- Visión general
- Requisitos
- Estructura del workspace
- Cómo levantar el proyecto
- Cómo ejecutar tests y quality checks
- Cómo ejecutar los examples
- Arquitectura y diagramas
- Buenas prácticas de desarrollo

---

## Visión general

`flow-chem` contiene varios crates organizados por responsabilidad:

- `crates/chem-domain` — modelos del dominio (Molecule, MoleculeFamily, Properties), ports y services.
- `crates/flow` — tipos y traits para persistencia de Flows (FlowRepository, FlowData, snapshots) y una implementación en memoria para tests.
- `crates/chem-persistence` — adaptadores Diesel para persistencia (SQLite/Postgres) e implementación de repositorios.
- `crates/chem-providers` — integraciones con motores químicos (RDKit vía Python/PyO3) y mocks para testing.
- `crates/chem-workflow` — definición de workflows concretos (CADMA) y pasos (steps).
- `crates/chem-utils` — utilidades y helpers para tests.
- `examples/` — ejemplos de uso (domain y flow).

---

## Requisitos

- Rust (stable, edition 2021) y toolchain estándar (rustup)
- Docker & docker-compose (recomendado para entornos con RDKit y Postgres)
- Python 3.11+ con RDKit si vas a usar `chem-providers` con la implementación real
- SQLite para pruebas locales (se usa por defecto en scripts de testing)

---

## Estructura del workspace (resumen)

```
flow-chem/
├─ Cargo.toml (workspace)
├─ crates/
│  ├─ chem-domain/
│  ├─ flow/
│  ├─ chem-persistence/
│  ├─ chem-providers/
│  ├─ chem-workflow/
│  └─ chem-utils/
├─ scripts/
└─ examples/
```

---

## Cómo levantar el proyecto (desarrollo)

Recomendación: usar Docker para reproducibilidad (RDKit + Postgres). Si prefieres local, instala las dependencias listadas arriba.

1. Levantar servicios con docker-compose (db + app-dev):

```bash
# Desde la raíz del repo
docker-compose up -d db app-dev
```

1. Acceder al contenedor de desarrollo (opcional):

```bash
# Abrir una shell dentro del contenedor app-dev
docker-compose exec app-dev bash
```

1. Ejecutar tests dentro del contenedor (usa RDKit si está disponible):

```bash
./scripts/run_tests_in_docker.sh
```

Si no quieres Docker y usas mocks RDKit, usa:

```bash
# Ejecutar tests con provider mock (no necesita RDKit/Python)
RUST_BACKTRACE=1 cargo test --workspace --all-targets --features mock_rdkit
```

---

## Comandos de calidad y desarrollo

Formateo, lint y tests (recomendado antes de subir cambios):

```bash
# Formatear todo
cargo fmt --all

# Lint (clippy) — falla en warnings
cargo clippy --workspace --all-targets -- -D warnings

# Ejecutar tests de todo el workspace
cargo test --workspace

# Comando único recomendado
cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

Scripts útiles (en `scripts/`):

- `./scripts/run_tests_in_docker.sh` — ejecuta tests dentro de contenedor dev (usa RDKit si está disponible)
- `./scripts/run_tests_with_mocks.sh` — ejecuta tests con `mock_rdkit` feature
- `./scripts/generate_coverage.sh` — genera reportes de cobertura (LCOV, Cobertura)
- `./scripts/run_sonar.sh` — wrapper para ejecutar SonarScanner (requiere token)

---

## Cómo ejecutar los examples

Hay ejemplos en `examples/` y en `crates/*/examples`.

1. Ejecutar ejemplo domain (`examples/example-domain.rs`):

```bash
cargo run --example example-domain --features testing
```

1. Ejecutar ejemplo flow (`examples/example-main.rs`):

```bash
cargo run --example example-main --features testing
```

1. Algunos crates tienen `examples/` propios (chem-persistence, chem-workflow). Para ejecutar el ejemplo de persistencia:

```bash
cd crates/chem-persistence
cargo run --example persistence_simple_usage --features sqlite
```

---

## Arquitectura y Diagramas

### Arquitectura general (hexagonal)

```mermaid
flowchart TB
  App[Aplicación / Examples]
  subgraph Workflow Layer
    Wf[chem-workflow]
    FlowEngine[flow]
  end
  Domain[chem-domain]
  subgraph Infra
    Persist[chem-persistence]
    Providers[chem-providers]
  end

  App --> Wf
  Wf --> Domain
  FlowEngine --> Domain
  Wf --> Persist
  Wf --> Providers
  Persist --> DB[(Database SQLite/Postgres)]
  Providers --> RDKit[(RDKit Python)]
```

### Diagrama de clases (resumen dominio)

```mermaid
classDiagram
  class Molecule {
    +inchikey: String
    +smiles: String
    +inchi: String
  }
  class MoleculeFamily {
    +id: UUID
    +molecules: Vec<Molecule>
    +family_hash: String
  }
  class MolecularProperty
  class FamilyProperty
  MoleculeFamily "1" *-- "*" Molecule
  MolecularProperty --> Molecule
  FamilyProperty --> MoleculeFamily
```

### Diagrama de flujo (ejecución de workflow)

```mermaid
flowchart TD
  Start[Inicio] --> Create[FlowEngine::start_flow]
  Create --> StepExec[Ejecutar pasos]
  StepExec --> Persist[Persistir FlowData]
  Persist --> Snapshot[Guardar snapshot]
  Snapshot --> End[Fin]
```

---

## Requerimientos y configuración ambiental

Variables de entorno importantes:

- `DATABASE_URL` o `CHEM_DB_URL` — URL de la base de datos (SQLite o Postgres)
- `PYO3_PYTHON` — ruta al ejecutable de Python si usas PyO3/Python bindings
- `SONAR_TOKEN` — token para SonarQube (solo CI)

Ejemplo para SQLite en memoria (útil para pruebas rápidas):

```bash
export DATABASE_URL="file:memdb1?mode=memory&cache=shared"
```

Para Postgres (ejemplo local Docker):

```bash
export DATABASE_URL="postgres://user:pass@localhost:5432/flow_chem"
```

---

## Buenas prácticas y guía de código

- Sigue principios SOLID: domain puro (`chem-domain`) sin dependencias de persistencia o providers.
- Usa traits (ports) y DI para inyectar adaptadores en `chem-persistence` y `chem-providers`.
- Mantén los steps de workflow pequeños y composables (single-responsibility).
- Escribe tests unitarios para lógica pura y tests de integración para repositorios y providers.
- Usa `mock_rdkit` para pruebas rápidas sin RDKit.

Estilo de commits y PR:

- Commits pequeños con un objetivo claro.
- PRs por feature/fase de refactorización.
- Incluir tests y actualizar README/documentación.

---

## Ayuda y contacto

Si tienes dudas o encuentras problemas, crea un issue en el repositorio o contacta a los mantenedores del proyecto.

---

Archivo generado automáticamente. Actualízalo con información adicional del proyecto según se avanza en la refactorización.
