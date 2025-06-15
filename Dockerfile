# Build stage
FROM rust:latest AS builder

WORKDIR /app

# Install required dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build the application
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies including curl for GCP metadata server access
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/solana-trading-bot /app/

# Copy Firestore configuration files
COPY firestore.rules /app/
COPY firestore.indexes.json /app/

# Set environment variables
ENV PORT=8080
ENV RUST_LOG=solana_trading_bot=info

# GCP authentication will use the default service account when running on Cloud Run
# For local development, mount the service account key file

# Expose port
EXPOSE 8080

# Run the application
CMD ["./solana-trading-bot"]