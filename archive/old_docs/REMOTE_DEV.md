# 远程开发服务器使用指南

## 服务器信息

- **地址**: `root@104.194.91.83`
- **端口**: `26974`
- **架构**: Linux x86_64 (AlmaLinux 9.5)
- **认证**: SSH Key（已配置免密登录）

## 开发流程

### 1. 本地修改代码
```bash
# 在本地修改代码
vim crates/memoryos-gateway/src/main.rs
```

### 2. 同步到远端
```bash
# 一键同步（使用 tar+scp）
./sync.sh
```

### 3. 远端编译运行
```bash
# 连接服务器
ssh root@104.194.91.83 -p 26974

# 进入目录
cd /opt/memoryos

# 编译
source ~/.cargo/env
cargo build --release

# 启动 Redis 和 Qdrant（依赖服务）
docker-compose -f docker-compose.middleware-demo.yml up -d redis qdrant

# 直接运行二进制（不用 Docker）
nohup ./target/release/memoryos-gateway > memoryos.log 2>&1 &

# 查看日志
tail -f memoryos.log

# 健康检查
curl http://localhost:8080/health
```

## 快捷命令

### 同步代码
```bash
./sync.sh
```

### SSH 连接
```bash
ssh root@104.194.91.83 -p 26974
```

### 远程编译
```bash
ssh root@104.194.91.83 -p 26974 'cd /opt/memoryos && source ~/.cargo/env && cargo build --release'
```

### 远程启动
```bash
ssh root@104.194.91.83 -p 26974 'cd /opt/memoryos && docker-compose -f docker-compose.middleware-demo.yml up -d redis qdrant && nohup ./target/release/memoryos-gateway > memoryos.log 2>&1 &'
```

### 远程停止
```bash
ssh root@104.194.91.83 -p 26974 'pkill memoryos-gateway'
```

### 查看日志
```bash
ssh root@104.194.91.83 -p 26974 'tail -f /opt/memoryos/memoryos.log'
```

### 健康检查
```bash
ssh root@104.194.91.83 -p 26974 'curl http://localhost:8080/health'
```

## 服务访问

- **Gateway**: http://104.194.91.83:8080
- **Health**: http://104.194.91.83:8080/health
- **Metrics**: http://104.194.91.83:8080/metrics

## 注意事项

1. **本地开发，远端运行**：本地只负责写代码，远端负责编译和运行
2. **架构差异**：本地 macOS (ARM64)，服务器 Linux (x86_64)，必须在服务器上编译
3. **不使用 Docker 运行主服务**：由于架构问题，主服务直接运行二进制，只用 Docker 运行 Redis 和 Qdrant
4. **自动同步**：使用 `./sync.sh` 快速同步代码（tar+scp 方式）
5. **环境变量**：首次部署需要在服务器上配置 `.env` 文件

## 首次部署

```bash
# 1. 同步代码
./sync.sh

# 2. 连接服务器
ssh root@104.194.91.83 -p 26974

# 3. 配置环境
cd /opt/memoryos
cp .env.example .env
vim .env  # 填入 API keys

# 4. 编译
source ~/.cargo/env
cargo build --release

# 5. 启动依赖服务
docker-compose -f docker-compose.middleware-demo.yml up -d redis qdrant

# 6. 启动主服务
nohup ./target/release/memoryos-gateway > memoryos.log 2>&1 &

# 7. 验证
curl http://localhost:8080/health
```

## 故障排查

### 服务无法启动
```bash
# 查看日志
ssh root@104.194.91.83 -p 26974 'tail -100 /opt/memoryos/memoryos.log'
```

### 重新编译
```bash
# 清理并重新编译
ssh root@104.194.91.83 -p 26974 'cd /opt/memoryos && source ~/.cargo/env && cargo clean && cargo build --release'
```

### 重启服务
```bash
# 停止
ssh root@104.194.91.83 -p 26974 'pkill memoryos-gateway'

# 启动
ssh root@104.194.91.83 -p 26974 'cd /opt/memoryos && nohup ./target/release/memoryos-gateway > memoryos.log 2>&1 &'
```

### 检查端口
```bash
# 检查 8080 端口是否被占用
ssh root@104.194.91.83 -p 26974 'netstat -tlnp | grep 8080'
```

## 架构说明

由于本地是 macOS ARM64，服务器是 Linux x86_64，采用以下方案：

- **本地**：开发代码，使用 `./sync.sh` 同步
- **服务器**：编译 x86_64 二进制，直接运行（不用 Docker 容器）
- **依赖服务**：Redis 和 Qdrant 使用 Docker 运行（官方镜像支持 x86_64）
