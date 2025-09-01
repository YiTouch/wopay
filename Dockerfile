# WoPay Web3支付系统Docker配置
# 多阶段构建，优化镜像大小和安全性

# 构建阶段
FROM rust:1.75-slim as builder

# 安装系统依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 复制Cargo文件
COPY Cargo.toml Cargo.lock ./

# 创建虚拟main.rs以缓存依赖
RUN mkdir src && echo "fn main() {}" > src/main.rs

# 构建依赖 (缓存层)
RUN cargo build --release && rm -rf src

# 复制源代码
COPY src ./src
COPY migrations ./migrations

# 构建应用
RUN cargo build --release

# 运行阶段
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*

# 创建应用用户
RUN useradd -m -u 1000 wopay

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/wopay ./
COPY --from=builder /app/migrations ./migrations

# 设置文件权限
RUN chown -R wopay:wopay /app
USER wopay

# 暴露端口
EXPOSE 8080

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# 启动应用
CMD ["./wopay"]
