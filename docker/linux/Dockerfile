FROM rust:1.76-slim

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

COPY ../../BinaryOptionsToolsV2 .

# Build .whl using maturin
CMD ["maturin", "build", "--release", "--target", "aarch64-linux-android", "--interpreter", "python3"]
