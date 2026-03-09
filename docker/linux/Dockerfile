# Stage 1: Planner
FROM lukemathwalker/cargo-chef:latest-rust-1.76-slim AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Builder
FROM lukemathwalker/cargo-chef:latest-rust-1.76-slim AS builder
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json

# Install system dependencies
RUN apt-get update && apt-get install -y \
    curl unzip build-essential pkg-config cmake \
    python3 python3-pip python3-venv \
    clang git libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create a virtualenv and install maturin inside
ENV VENV_PATH=/opt/venv
RUN python3 -m venv $VENV_PATH
ENV PATH="$VENV_PATH/bin:$PATH"

RUN pip install --upgrade pip && pip install maturin

# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# Build the application
COPY . .
# We use maturin to build the Python package
# Using --strip to reduce binary size
RUN maturin build --release --strip --interpreter python3

# Stage 3: Runner (Minimal image)
FROM debian:bookworm-slim AS runner
WORKDIR /app
COPY --from=builder /app/target/wheels/*.whl /app/
# The runner stage just holds the artifacts in this case as it's a build container.
# If it were an app, we would install the wheel here.
CMD ["ls", "-l", "/app"]
