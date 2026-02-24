# MemoryOS Gateway Dockerfile
FROM rust:1.85-slim-bookworm AS chef

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-chef --locked

WORKDIR /build

# Phase 1: Generate dependency recipe (changes only when Cargo.toml/lock change)
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo chef prepare --recipe-path recipe.json

# Phase 2: Build dependencies (cached unless recipe changes)
FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Phase 3: Build application (only recompiles your code)
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
RUN cargo build --release --bin memoryos-gateway

# 运行时镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /build/target/release/memoryos-gateway /app/

# 暴露端口
EXPOSE 8080

# 运行
CMD ["./memoryos-gateway"]
