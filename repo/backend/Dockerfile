# Multi-stage build: compile the backend binary, then ship a slim runtime.

FROM rust:1.88-bookworm AS builder
WORKDIR /build
COPY . .
# Cache the cargo registry and target dir between builds to speed up rebuilds.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --package backend && \
    cp target/release/backend /backend

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /backend /app/backend
COPY config /app/config
COPY db /app/db
COPY backend/docker-entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh \
    && mkdir -p /app/data/review_images /app/data/report_attachments /app/data/face_images

EXPOSE 8000
HEALTHCHECK --interval=10s --timeout=3s --start-period=15s --retries=5 \
    CMD curl -fsS http://localhost:8000/api/health || exit 1

ENTRYPOINT ["/entrypoint.sh"]
