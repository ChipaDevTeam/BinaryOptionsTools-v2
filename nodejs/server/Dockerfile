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
COPY Cargo.toml ./
COPY crates ./crates
RUN cargo build -p binary_options_tools_napi --release

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
