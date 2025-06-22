FROM rust:1.70 as builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin file-processor

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/file-processor /app/file-processor

CMD ["./file-processor"] 