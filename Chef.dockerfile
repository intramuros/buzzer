FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
RUN apt-get update && apt install lld clang -y

WORKDIR /app
FROM chef AS server-planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json
FROM chef AS server-builder
COPY --from=server-planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
# Up to this point, if our dependency tree stays the same,
# all layers should be cached.
COPY . .
RUN cargo build --release -p server

FROM rustlang/rust:nightly AS frontend-builder
RUN rustup target add wasm32-unknown-unknown
SHELL [ "/bin/bash", "-c" ]
RUN curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
RUN cargo binstall dioxus-cli
WORKDIR /app
COPY . .
RUN dx bundle --out-dir dist -p frontend --platform web


FROM debian:stable-slim
WORKDIR /app
ENV APP_ENVIRONMENT="production"

COPY --from=server-builder /app/target/release/server server
COPY ./server/configuration configuration

COPY --from=frontend-builder /app/dist/public dist

# Expose the port the server will run on
EXPOSE 8080

# Set the command to run the server
CMD ["./server"]
