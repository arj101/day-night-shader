FROM lukemathwalker/cargo-chef:latest-rust-slim-bookworm AS chef
WORKDIR /day-night-shader

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
RUN apt-get update && apt-get install libsfml-dev build-essential -y && rm -rf /var/lib/apt/lists/*
COPY --from=planner /day-night-shader/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --bin day-night-shader-native

FROM debian:bookworm-slim AS runtime
WORKDIR /day-night-shader
RUN apt-get update && apt-get install libsfml-dev xvfb -y && rm -rf /var/lib/apt/lists/*
COPY --from=builder /day-night-shader/target/release/day-night-shader-native /usr/local/bin
ENTRYPOINT ["xvfb-run", "day-night-shader-native"]
