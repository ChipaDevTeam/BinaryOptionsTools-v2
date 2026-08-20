# Build the Linux wheel with maturin inside a manylinux-compatible image.
# Build from the repository root:
#   docker build -f docker/linux/Dockerfile --target export -o dist/linux .
FROM ghcr.io/pyo3/maturin:latest AS builder

# Some transitive crates (aws-lc-rs via rustls) need cmake/clang to build
RUN yum install -y cmake clang || (apt-get update && apt-get install -y cmake clang) || true

WORKDIR /io
COPY . .

WORKDIR /io/python
RUN maturin build --release --strip -o /io/dist

# Export stage: contains only the built wheel(s)
FROM scratch AS export
COPY --from=builder /io/dist/*.whl /
