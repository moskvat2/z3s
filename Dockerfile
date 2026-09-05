# ==============================================================================
# Stage 1: Build Environment (Musl Static Target)
# ==============================================================================
FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev build-base pkgconfig

WORKDIR /usr/src/z3s

# Copy Cargo configurations and manifests for caching dependencies
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY bin ./bin

# Build static release binary
RUN cargo build --release --bin z3s-server

# ==============================================================================
# Stage 2: Minimal Production Runtime
# ==============================================================================
FROM alpine:3.20 AS runtime

RUN apk add --no-cache ca-certificates tzdata wget \
    && addgroup -g 10001 z3s \
    && adduser -D -u 10001 -G z3s -h /data -s /bin/sh z3s \
    && mkdir -p /data /var/log/z3s \
    && chown -R z3s:z3s /data /var/log/z3s

COPY --from=builder /usr/src/z3s/target/release/z3s-server /usr/local/bin/z3s-server

# S3 Gateway REST API Port
EXPOSE 9000

# Persistent Data Storage Volume
VOLUME ["/data"]

# Default Environment Variables
ENV Z3S_BIND="0.0.0.0:9000" \
    Z3S_DATA_DIR="/data" \
    Z3S_ACCESS_KEY="Z3SACCESSKEYEXAMPLE" \
    Z3S_SECRET_KEY="Z3SSECRETKEYEXAMPLE1234567890ABCDEF" \
    Z3S_DATA_SHARDS="4" \
    Z3S_PARITY_SHARDS="2"

USER z3s

# Healthcheck to verify S3 API responsiveness
HEALTHCHECK --interval=15s --timeout=5s --start-period=5s --retries=3 \
    CMD wget -q -O /dev/null http://127.0.0.1:9000/ || exit 1

ENTRYPOINT ["/usr/local/bin/z3s-server"]
