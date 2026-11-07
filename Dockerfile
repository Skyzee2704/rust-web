# syntax=docker/dockerfile:1
# Rust 1.99+ is required by current crate editions (hyper-util, time, tokio).
FROM rust:1.99-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY static ./static
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/letheans /usr/local/bin/letheans
COPY --from=build /app/static /app/static
ENV PORT=3000
EXPOSE 3000
USER nobody
CMD ["letheans"]
