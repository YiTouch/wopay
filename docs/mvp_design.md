# WoPay MVP版本设计

## 1. MVP功能范围

### 1.1 核心功能 (Must Have)
- **商户注册**: 基础商户信息管理
- **API密钥管理**: 生成和管理API凭证
- **支付订单创建**: 创建区块链支付订单
- **Ethereum支付**: 支持ETH和USDT支付
- **交易监听**: 实时监听支付状态
- **Webhook通知**: 支付状态回调
- **基础管理后台**: 商户和订单管理

### 1.2 简化功能 (Should Have)
- **单一区块链**: 仅支持Ethereum主网
- **有限代币**: 仅支持ETH和USDT
- **基础安全**: API密钥认证，HTTPS传输
- **简单UI**: 基础的管理界面
- **基础监控**: 简单的日志和错误监控

### 1.3 暂缓功能 (Won't Have)
- ❌ Solana和BSC支持
- ❌ 多重签名钱包
- ❌ 冷热钱包分离
- ❌ 高级风控系统
- ❌ 复杂的财务报表
- ❌ 多语言SDK

## 2. MVP技术架构

### 2.1 简化架构
```
┌─────────────────────────────────────┐
│            第三方应用                │
└─────────────────┬───────────────────┘
                  │ HTTP API
┌─────────────────▼───────────────────┐
│            API网关                  │
│         (Actix-web)                │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│           业务服务层                 │
│  ┌─────────┬─────────┬─────────┐    │
│  │商户服务  │支付服务  │钱包服务  │    │
│  └─────────┴─────────┴─────────┘    │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│          Ethereum适配器              │
│         (ethers-rs)                │
└─────────────────┬───────────────────┘
                  │
┌─────────────────▼───────────────────┐
│          Ethereum网络               │
└─────────────────────────────────────┘
```

### 2.2 技术栈
- **后端**: Rust + Actix-web + PostgreSQL
- **前端**: Vue3 + TypeScript + Element Plus
- **区块链**: ethers-rs (仅Ethereum)
- **数据库**: PostgreSQL (单实例)
- **部署**: Docker + Docker Compose

## 3. 核心API设计

### 3.1 创建支付订单
```http
POST /api/v1/payments
Authorization: Bearer {api_key}
Content-Type: application/json

{
    "order_id": "ORDER_123456",
    "amount": "100.50",
    "currency": "USDT",
    "callback_url": "https://merchant.com/webhook",
    "expires_in": 3600
}
```

**响应**:
```json
{
    "success": true,
    "data": {
        "payment_id": "uuid",
        "payment_address": "0x742d35Cc6634C0532925a3b8D4C9db96DfbBb8b2",
        "amount": "100.50",
        "currency": "USDT",
        "expires_at": "2024-01-01T13:00:00Z",
        "qr_code": "data:image/png;base64,..."
    }
}
```

### 3.2 查询支付状态
```http
GET /api/v1/payments/{payment_id}
Authorization: Bearer {api_key}
```

### 3.3 Webhook回调
```json
{
    "event": "payment.completed",
    "payment_id": "uuid",
    "order_id": "ORDER_123456",
    "status": "completed",
    "amount": "100.50",
    "currency": "USDT",
    "transaction_hash": "0x...",
    "confirmations": 12,
    "timestamp": "2024-01-01T12:05:00Z",
    "signature": "hmac_sha256_signature"
}
```

## 4. 数据库设计 (简化版)

```sql
-- 商户表
CREATE TABLE merchants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    api_key VARCHAR(255) UNIQUE NOT NULL,
    api_secret VARCHAR(255) NOT NULL,
    webhook_url VARCHAR(500),
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT NOW()
);

-- 支付订单表
CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id UUID REFERENCES merchants(id),
    order_id VARCHAR(255) NOT NULL,
    amount DECIMAL(36,18) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    payment_address VARCHAR(42) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    transaction_hash VARCHAR(66),
    confirmations INTEGER DEFAULT 0,
    expires_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(merchant_id, order_id)
);
```

## 5. 部署配置

### 5.1 Docker Compose
```yaml
version: '3.8'

services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: wopay_mvp
      POSTGRES_USER: wopay
      POSTGRES_PASSWORD: password123
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

  backend:
    build: .
    environment:
      DATABASE_URL: postgres://wopay:password123@postgres:5432/wopay_mvp
      ETHEREUM_RPC_URL: https://eth-mainnet.alchemyapi.io/v2/your-api-key
      RUST_LOG: info
    ports:
      - "8080:8080"
    depends_on:
      - postgres

  frontend:
    build:
      context: ./frontend
      dockerfile: Dockerfile
    ports:
      - "3000:80"
    depends_on:
      - backend

volumes:
  postgres_data:
```

## 6. MVP发布计划

### 6.1 开发阶段 (4周)
- **Week 1**: 后端核心功能开发
- **Week 2**: 前端界面开发
- **Week 3**: 集成测试和调试
- **Week 4**: 部署和文档完善

### 6.2 测试阶段 (2周)
- **内部测试**: 功能测试、性能测试
- **用户测试**: 邀请早期用户测试
- **安全测试**: 基础安全检查

### 6.3 发布阶段 (1周)
- **生产部署**: 正式环境部署
- **监控配置**: 基础监控告警
- **文档发布**: API文档和使用指南

## 7. 成功指标

### 7.1 技术指标
- **系统可用性**: ≥99%
- **API响应时间**: ≤1秒
- **支付成功率**: ≥95%

### 7.2 业务指标
- **注册商户**: 10个
- **完成支付**: 100笔
- **交易金额**: $10,000

### 7.3 用户反馈
- **易用性评分**: ≥4.0/5.0
- **文档质量**: ≥4.0/5.0
- **技术支持**: ≥4.0/5.0

这个MVP版本专注于核心功能，为后续完整版本奠定基础。
