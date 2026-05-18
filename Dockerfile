FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY dav-server ./dav-server
COPY client ./client
COPY connectors ./connectors
COPY sync-dispatcher ./sync-dispatcher
COPY sync-worker ./sync-worker

RUN cargo build -p foliofs-dav-server --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /data

COPY --from=builder /app/target/release/foliofs-dav-server /usr/local/bin/foliofs-dav-server

EXPOSE 4918

ENTRYPOINT ["foliofs-dav-server"]
CMD ["--bind", "0.0.0.0:4918", "--root", "/data"]
