# Crate `chem-providers` — wrappers para motores químicos (en español)
`chem-providers` contiene adaptadores y bindings para interactuar con motores
químicos externos (por ejemplo RDKit) desde Rust. En este repositorio se
incluye un envoltorio Python (`python/rdkit_wrapper.py`) y bindings `pyo3`
para llamar a esas funciones desde Rust.
Requisitos
- Python 3 con RDKit instalado si quieres usar funcionalidades reales.
- `pyo3` está configurado en el `Cargo.toml` y espera que Python esté
  disponible en el entorno (puedes controlar la ruta con `PYO3_PYTHON`).
Uso
Desde el workspace puedes compilar y ejecutar ejemplos que dependan de
`chem-providers` normalmente con `cargo build` o `cargo run`.
Notas
- Para desarrollo local con RDKit puedes crear un entorno Conda y exportar
  `PYO3_PYTHON` apuntando al ejecutable Python de ese entorno.
