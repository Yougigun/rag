FROM rust:latest AS builder

WORKDIR /app
COPY . .
RUN cargo build --release --bin rag-api

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/rag-api /app/rag-api

EXPOSE 3000
CMD ["./rag-api"] 