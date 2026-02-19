# MemoryOS Gateway Dockerfile
FROM rust:1.83-slim AS builder

WORKDIR /build

# 安装依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制源代码
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# 构建 release 版本
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
