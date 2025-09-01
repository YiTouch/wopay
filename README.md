# WoPay - Web3支付系统 💰

一个基于Rust构建的现代化Web3支付网关，支持以太坊区块链支付，为第三方应用提供简单、安全、可靠的加密货币支付解决方案。

## 🚀 特性

- **多币种支持**: ETH、USDT等主流加密货币
- **实时监听**: 自动监听区块链交易状态
- **安全可靠**: HMAC签名验证、API密钥认证
- **Webhook通知**: 支付状态变更实时通知
- **高性能**: 基于Rust + Actix-web构建
- **易于集成**: RESTful API，支持多种编程语言
- **Docker部署**: 容器化部署，支持水平扩展

## 🏗️ 技术架构

- **后端**: Rust + Actix-web + SQLx
- **数据库**: PostgreSQL + Redis
- **区块链**: Ethereum + Ethers-rs
- **部署**: Docker + Docker Compose + Nginx

## 📦 快速开始

### 环境要求

- Rust 1.75+
- PostgreSQL 13+
- Redis 6+
- Docker & Docker Compose (可选)

### 本地开发

1. **克隆项目**
```bash
git clone https://github.com/your-org/wopay.git
cd wopay
```

2. **配置环境变量**
```bash
cp .env.example .env
# 编辑.env文件，填入实际配置
```

3. **启动数据库**
```bash
docker-compose up -d postgres redis
```

4. **运行迁移**
```bash
cargo install sqlx-cli
sqlx migrate run
```

5. **启动服务**
```bash
cargo run
```

服务将在 `http://localhost:8080` 启动。

### Docker部署

```bash
# 构建并启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f wopay
```

## 🔧 配置说明

### 环境变量

| 变量名 | 说明 | 默认值 |
|--------|------|--------|
| `SERVER_HOST` | 服务器监听地址 | `127.0.0.1` |
| `SERVER_PORT` | 服务器端口 | `8080` |
| `DATABASE_URL` | PostgreSQL连接URL | - |
| `ETHEREUM_RPC_URL` | 以太坊RPC节点URL | - |
| `ETHEREUM_WS_URL` | 以太坊WebSocket URL | - |
| `CHAIN_ID` | 链ID (1=主网, 5=Goerli) | `1` |
| `JWT_SECRET` | JWT密钥 | - |

完整配置请参考 `.env.example` 文件。

## 📚 API文档

### 认证

所有API请求需要在请求头中包含API密钥：

```http
X-API-Key: your_api_key_here
```

### 主要接口

#### 1. 商户注册
```http
POST /api/v1/merchants
```

#### 2. 创建支付订单
```http
POST /api/v1/payments
```

#### 3. 查询支付状态
```http
GET /api/v1/payments/{payment_id}
```

详细API文档请参考 [API Documentation](docs/api_documentation.md)。

## 🔌 集成示例

### JavaScript/Node.js

```javascript
const WoPaySDK = require('./wopay-sdk');

const wopay = new WoPaySDK('your_api_key', 'your_api_secret');

// 创建支付
const payment = await wopay.createPayment({
  order_id: 'ORDER_001',
  amount: '99.99',
  currency: 'USDT',
  expires_in: 3600
});

console.log('支付地址:', payment.data.payment_address);
```

### PHP

```php
$wopay = new WoPaySDK('your_api_key', 'your_api_secret');

$payment = $wopay->createPayment([
    'order_id' => 'ORDER_001',
    'amount' => '99.99',
    'currency' => 'USDT',
    'expires_in' => 3600
]);

echo "支付地址: " . $payment['data']['payment_address'];
```

更多示例请参考 [Usage Examples](docs/usage_examples.md)。

## 🔒 安全特性

- **API密钥认证**: 每个商户拥有唯一的API密钥对
- **HMAC签名**: Webhook通知使用HMAC-SHA256签名验证
- **请求限流**: 防止API滥用和DDoS攻击
- **数据加密**: 敏感数据加密存储
- **审计日志**: 完整的操作日志记录

## 🔄 支付流程

1. **商户注册**: 获取API密钥
2. **创建订单**: 调用API创建支付订单
3. **用户支付**: 用户扫码或复制地址进行支付
4. **交易监听**: 系统自动监听区块链交易
5. **状态更新**: 交易确认后更新支付状态
6. **Webhook通知**: 向商户发送状态变更通知

## 📊 支持的币种

| 币种 | 符号 | 网络 | 合约地址 |
|------|------|------|----------|
| 以太坊 | ETH | Ethereum | - |
| USDT | USDT | Ethereum | 0xdAC17F958D2ee523a2206206994597C13D831ec7 |

## 🧪 测试

### 运行单元测试

```bash
cargo test
```

### 运行集成测试

```bash
cargo test --test integration
```

### 测试网环境

使用Goerli测试网进行开发测试：

```bash
# 设置测试网配置
export CHAIN_ID=5
export ETHEREUM_RPC_URL=https://eth-goerli.alchemyapi.io/v2/YOUR_API_KEY
```

## 📈 监控和运维

### 健康检查

```bash
curl http://localhost:8080/health
```

### 系统状态

```bash
curl http://localhost:8080/api/v1/status
```

### 日志查看

```bash
# Docker环境
docker-compose logs -f wopay

# 本地环境
RUST_LOG=debug cargo run
```

## 🤝 贡献指南

1. Fork项目
2. 创建功能分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建Pull Request

## 📄 许可证

本项目采用MIT许可证。详情请参考 [LICENSE](LICENSE) 文件。

## 🆘 支持

- **文档**: [docs/](docs/)
- **问题反馈**: [GitHub Issues](https://github.com/your-org/wopay/issues)
- **邮箱**: support@wopay.com

## 🗺️ 路线图

- [x] MVP版本 (Ethereum支持)
- [ ] Solana区块链集成
- [ ] BSC (Binance Smart Chain) 支持
- [ ] 多重签名钱包
- [ ] 手续费管理
- [ ] 商户仪表板
- [ ] 移动端SDK
- [ ] 高级分析报告

## 📊 项目状态

![Build Status](https://img.shields.io/badge/build-passing-brightgreen)
![Coverage](https://img.shields.io/badge/coverage-85%25-green)
![Version](https://img.shields.io/badge/version-1.0.0-blue)
![License](https://img.shields.io/badge/license-MIT-blue)

---

**WoPay** - 让Web3支付变得简单 🚀
