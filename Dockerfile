# Dockerfile del proyecto: proporciona Python (RDKit vía conda) y el toolchain de Rust; compila el workspace
FROM condaforge/mambaforge:latest AS base

ENV DEBIAN_FRONTEND=noninteractive \
    PYTHONUNBUFFERED=1 \
    PYO3_PYTHON=/opt/conda/bin/python \
    PYTHON_SYS_EXECUTABLE=/opt/conda/bin/python \
    LD_LIBRARY_PATH=/opt/conda/lib

WORKDIR /workspace

# Install system deps for building Rust crates that may require libssl, pkg-config, etc.
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    curl \
    pkg-config \
    libssl-dev \
    libpq-dev \
    git \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Install RDKit via conda (conda-forge)
RUN mamba install -y -c conda-forge "python=3.11" rdkit

# Instala rustup y el toolchain de Rust
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable && \
    /root/.cargo/bin/rustup default stable && \
    /root/.cargo/bin/rustup target add x86_64-unknown-linux-gnu || true

# Añade el binario de cargo al PATH
ENV PATH=/root/.cargo/bin:$PATH

# Instala el toolchain nightly y `cargo-tarpaulin` para evitar instalaciones en
# tiempo de ejecución dentro del contenedor de cobertura. `cargo-tarpaulin`
# suele requerir nightly; instalarlo aquí evita que `cargo install` dispare
# descargas de rustup en tiempo de ejecución.
RUN /root/.cargo/bin/rustup toolchain install nightly || true && \
    /root/.cargo/bin/rustup run nightly /root/.cargo/bin/cargo install cargo-tarpaulin --locked || true

# Copy only manifests first to leverage Docker layer cache for dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates/chem-domain/Cargo.toml crates/chem-domain/Cargo.toml
COPY crates/flow/Cargo.toml crates/flow/Cargo.toml
COPY crates/chem-persistence/Cargo.toml crates/chem-persistence/Cargo.toml
COPY crates/chem-providers/Cargo.toml crates/chem-providers/Cargo.toml
COPY crates/chem-providers/requirements.txt crates/chem-providers/requirements.txt

# Pre-descarga dependencias de cargo (útil para cachear descargas del registry/git)
RUN cargo fetch || true

# Instalar requirements de Python ahora que requirements.txt está presente
RUN /opt/conda/bin/pip install --no-cache-dir -r crates/chem-providers/requirements.txt || true

# Observación: separamos la etapa "base" (toolchain + dependencias) de la
# etapa "builder" (compilación). La imagen de desarrollo/coverage debería
# usar la etapa `base` para no compilar el workspace durante su construcción
# (tarpaulin/tests compilarán bajo demanda dentro del contenedor). Esto
# preserva la caché de Docker para pasos pesados como apt/mamba/rustup/cargo fetch.

## Builder stage (compilation) ------------------------------
FROM base AS builder

# Copiar el resto del workspace y compilar en una etapa separada.
# Mantener la compilación fuera del `base` para que el dev image no
# ejecute `cargo build` durante su construcción.
COPY . /workspace

# Allow selecting cargo features at build time (e.g. pg_demo)
ARG FEATURES=""
# Pre-build: build the Rust workspace in release mode with optional features
# If FEATURES is empty, cargo will ignore --features flag.
RUN if [ -n "$FEATURES" ]; then \
            echo "Building with features: $FEATURES"; \
            cargo build --release --features "$FEATURES" || cargo build --features "$FEATURES"; \
        else \
            cargo build --release || cargo build; \
        fi

# Imagen final: imagen de runtime más ligera
FROM condaforge/mambaforge:latest
ENV PYO3_PYTHON=/opt/conda/bin/python \
    PYTHON_SYS_EXECUTABLE=/opt/conda/bin/python \
    LD_LIBRARY_PATH=/opt/conda/lib \
    PATH=/root/.cargo/bin:$PATH

WORKDIR /app

# Copy conda env and built binary from builder
COPY --from=builder /opt/conda /opt/conda
COPY --from=builder /workspace/target/release/main-core /app/main-core
COPY --from=builder /workspace/crates/chem-providers/python /app/python
COPY --from=builder /workspace/crates/chem-providers/requirements.txt /app/requirements.txt

# Copy entrypoint script and make executable
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh
# Ensure libpq runtime is available for Postgres connections
RUN apt-get update && apt-get install -y --no-install-recommends libpq5 && rm -rf /var/lib/apt/lists/* || true

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
