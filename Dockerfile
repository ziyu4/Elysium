FROM rust:1-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    libssl-dev \
    pkg-config \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTFLAGS="-C target-cpu=znver3"
    
WORKDIR /app

COPY . .

RUN cargo build --release

FROM gcr.io/distroless/cc-debian12 AS runner

COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

COPY --from=builder /usr/lib/x86_64-linux-gnu/libssl.so.3 /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libcrypto.so.3 /usr/lib/x86_64-linux-gnu/

WORKDIR /app

COPY --from=builder /app/target/release/elysium /app/elysium

USER 65534:65534

EXPOSE 8443

ENTRYPOINT ["/app/elysium"]