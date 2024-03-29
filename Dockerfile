FROM rust:slim AS builder

RUN set -eux; \
    apt-get update; \
    apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    libclang-dev \
    build-essential && \
    rustup component add rustfmt

WORKDIR /app

COPY . .

RUN cargo build -r

FROM debian:bullseye-slim

RUN set -eux; \
      apt-get update; \
      apt-get install -y --no-install-recommends \
      ca-certificates

COPY --from=builder /app/target/release/pigeon /usr/bin/pigeon

VOLUME /pigeon

ENTRYPOINT [ "/usr/bin/pigeon" ]
