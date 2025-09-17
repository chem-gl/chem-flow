# Project Dockerfile: provides Python (RDKit via conda) and Rust toolchain, builds workspace
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
RUN mamba install -y -c conda-forge "python=3.11" rdkit && \
    /opt/conda/bin/pip install --no-cache-dir -r /workspace/crates/chem-providers/requirements.txt || true

# Install rustup and Rust toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable && \
    /root/.cargo/bin/rustup default stable && \
    /root/.cargo/bin/rustup target add x86_64-unknown-linux-gnu || true

# Add cargo bin to PATH
ENV PATH=/root/.cargo/bin:$PATH

# Copy workspace
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

# Final image: smaller runtime image
FROM condaforge/mambaforge:latest
ENV PYO3_PYTHON=/opt/conda/bin/python \
    PYTHON_SYS_EXECUTABLE=/opt/conda/bin/python \
    LD_LIBRARY_PATH=/opt/conda/lib \
    PATH=/root/.cargo/bin:$PATH

WORKDIR /app

# Copy conda env and built binary from builder
COPY --from=base /opt/conda /opt/conda
COPY --from=base /workspace/target/release/main-core /app/main-core
COPY --from=base /workspace/crates/chem-providers/python /app/python
COPY --from=base /workspace/crates/chem-providers/requirements.txt /app/requirements.txt

# Copy entrypoint script and make executable
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh
# Ensure libpq runtime is available for Postgres connections
RUN apt-get update && apt-get install -y --no-install-recommends libpq5 && rm -rf /var/lib/apt/lists/* || true

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
