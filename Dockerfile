# Build stage
FROM rust:slim AS builder

# Install build dependencies and nightly toolchain
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/* && \
    rustup default nightly

# Create a new empty shell project
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the actual source code
COPY src ./src

# Build for release
# Touch main.rs to force rebuild of the app
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Create directories for input and output
RUN mkdir -p /input /output

# Copy the binary from builder
COPY --from=builder /app/target/release/RustCityGML2OBJ /usr/local/bin/citygml2obj

# Set the working directory
WORKDIR /data

# Set the entrypoint
ENTRYPOINT ["citygml2obj"]

# Default command (show help)
CMD ["--help"]
