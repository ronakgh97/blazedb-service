FROM rust:1.92-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy shit
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --bin blz_service

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from builder
COPY --from=builder /app/target/release/blz_service /app/blz_service

# Create directories upfront
RUN mkdir -p /home/blz_service/data /home/blz_service/logs /home/blz_service/billings

ENV RUST_LOG=info
ENV HOME=/home/blaze-service

EXPOSE 3000

# Health check - verify the binary exists
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s CMD test -f /app/blz_service || exit 1

CMD ["/app/blz_service"]