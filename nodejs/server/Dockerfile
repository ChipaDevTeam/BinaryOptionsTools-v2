# Container image for the bindings console.
#
# Build from the repository root, not from this directory — the Rust crates and
# the nodejs package both have to be in the context:
#
#   docker build -f nodejs/server/Dockerfile -t binary-options-console .
#   docker run --rm -p 8080:8080 -e AUTH_TOKEN=$(openssl rand -hex 24) binary-options-console
#
# The runtime stage must stay on the same glibc as the builder, so both are
# bookworm. Only the compiled addon crosses between them.

FROM rust:1-bookworm AS build
WORKDIR /src

# Dependency layer: manifests plus empty stub sources only. A source edit does not
# reach this layer, so the dependency crates stay cached by the GHA layer cache
# that console-image.yml already exports (cache-to: type=gha,mode=max).
# Cargo.lock is gitignored in this repository, so it cannot be copied in; the first
# build resolves it and the generated lock stays in the image for the layer below.
COPY Cargo.toml ./
COPY crates/core/Cargo.toml crates/core/Cargo.toml
COPY crates/core/macros/Cargo.toml crates/core/macros/Cargo.toml
COPY crates/macros/Cargo.toml crates/macros/Cargo.toml
COPY crates/binary_options_tools/Cargo.toml crates/binary_options_tools/Cargo.toml
COPY crates/bindings_napi/Cargo.toml crates/bindings_napi/Cargo.toml
COPY crates/bindings_pyo3/Cargo.toml crates/bindings_pyo3/Cargo.toml
COPY crates/bindings_uniffi/Cargo.toml crates/bindings_uniffi/Cargo.toml
COPY crates/bindings_uniffi/bo2_macros/Cargo.toml crates/bindings_uniffi/bo2_macros/Cargo.toml
COPY crates/bindings_uniffi/uniffi-bindgen/Cargo.toml crates/bindings_uniffi/uniffi-bindgen/Cargo.toml
RUN set -eux; \
    # Every member of the workspace above needs its manifest here, otherwise cargo
    # refuses to load the workspace at all. Fail with that sentence instead of
    # cargo's "failed to load manifest for workspace member" when a member is added.
    for d in $(sed -n '/^members = \[/,/^\]/p' Cargo.toml | grep -o '"[^"]*"' | tr -d '"'); do \
      test -f "$d/Cargo.toml" || { \
        echo "workspace member $d has no manifest in this layer: add a COPY line above"; exit 1; \
      }; \
      mkdir -p "$d/src"; : > "$d/src/lib.rs"; \
    done; \
    cargo build -p binary_options_tools_napi --release

COPY crates ./crates
# COPY keeps the mtimes from the build context, which are older than the stub
# artifacts from the layer above, so cargo would deem the workspace crates fresh
# and link the empty stub rlibs. Touch them so cargo rebuilds the workspace crates
# while the third-party dependencies above stay cached.
RUN find crates -type f -exec touch {} + \
 && cargo build -p binary_options_tools_napi --release

FROM node:22-bookworm-slim
WORKDIR /app/nodejs

COPY --from=build /src/target/release/libbinary_options_tools_napi.so ./binary-options-tools.node
COPY nodejs/package.json nodejs/index.js nodejs/index.d.ts ./
COPY nodejs/server ./server

# 0.0.0.0 because a container that binds loopback is only reachable from
# inside itself, which looks exactly like a broken deploy.
ENV HOST=0.0.0.0 \
    PORT=8080 \
    NODE_ENV=production

EXPOSE 8080
USER node
CMD ["node", "server/server.js"]
