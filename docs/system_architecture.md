# WoPay Web3支付系统 - 系统架构设计

## 1. 系统概述

WoPay是一个支持多区块链的Web3支付网关系统，允许第三方应用轻松集成区块链支付功能。系统支持ETH、Solana、BSC等主流区块链，提供完整的支付流程管理、商户管理和资金结算服务。

### 1.1 核心特性
- **多链支持**: ETH、Solana、BSC等主流区块链
- **第三方集成**: 提供RESTful API和SDK
- **实时监控**: 区块链交易状态实时追踪
- **安全保障**: 多重签名、HMAC验证、资金托管
- **高可用性**: 微服务架构、负载均衡、故障转移

## 2. 整体架构

### 2.1 系统架构图
```
┌─────────────────────────────────────────────────────────────┐
│                    第三方应用层                              │
├─────────────────────────────────────────────────────────────┤
│  电商平台  │  游戏平台  │  DeFi应用  │  其他Web3应用        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    API网关层                                │
├─────────────────────────────────────────────────────────────┤
│  认证授权  │  限流控制  │  请求路由  │  监控日志            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    业务服务层                                │
├─────────────────────────────────────────────────────────────┤
│ 商户服务 │ 支付服务 │ 钱包服务 │ 通知服务 │ 结算服务        │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    区块链适配层                              │
├─────────────────────────────────────────────────────────────┤
│  ETH适配器  │  Solana适配器  │  BSC适配器  │  通用适配器    │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    区块链网络                                │
├─────────────────────────────────────────────────────────────┤
│   Ethereum   │    Solana     │      BSC      │   其他链     │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 技术栈选型

#### 后端技术栈
- **语言**: Rust (2024 edition)
- **Web框架**: Actix-web 4.x
- **数据库**: PostgreSQL 15+ (主库) + Redis (缓存)
- **区块链库**: 
  - Ethers-rs (Ethereum/BSC)
  - Solana-client (Solana)
- **消息队列**: RabbitMQ
- **监控**: Prometheus + Grafana
- **日志**: tracing + serde_json

#### 前端技术栈
- **框架**: Vue 3 + TypeScript
- **状态管理**: Pinia
- **UI组件**: Element Plus
- **构建工具**: Vite
- **路由**: Vue Router 4
- **HTTP客户端**: Axios

#### 基础设施
- **容器化**: Docker + Docker Compose
- **编排**: Kubernetes (生产环境)
- **负载均衡**: Nginx
- **CI/CD**: GitHub Actions
- **云服务**: AWS/阿里云

## 3. 微服务架构设计

### 3.1 服务拆分

#### 3.1.1 商户服务 (Merchant Service)
- **职责**: 商户注册、认证、API密钥管理
- **端口**: 8001
- **数据库**: merchants表

#### 3.1.2 支付服务 (Payment Service)
- **职责**: 支付订单创建、状态管理、金额验证
- **端口**: 8002
- **数据库**: payments表

#### 3.1.3 钱包服务 (Wallet Service)
- **职责**: 钱包地址生成、私钥管理、交易签名
- **端口**: 8003
- **数据库**: wallets表

#### 3.1.4 区块链服务 (Blockchain Service)
- **职责**: 区块链交易监听、确认验证、多链支持
- **端口**: 8004
- **数据库**: blockchain_transactions表

#### 3.1.5 通知服务 (Notification Service)
- **职责**: Webhook推送、重试机制、消息队列
- **端口**: 8005
- **数据库**: webhook_logs表

#### 3.1.6 结算服务 (Settlement Service)
- **职责**: 资金结算、手续费计算、财务报表
- **端口**: 8006
- **数据库**: settlements表

### 3.2 服务间通信
- **同步通信**: HTTP/gRPC
- **异步通信**: RabbitMQ消息队列
- **服务发现**: Consul/etcd
- **配置管理**: 环境变量 + 配置中心

## 4. 数据库设计

### 4.1 核心表结构

#### 商户表 (merchants)
```sql
CREATE TABLE merchants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    api_key VARCHAR(255) UNIQUE NOT NULL,
    api_secret VARCHAR(255) NOT NULL,
    webhook_url VARCHAR(500),
    status VARCHAR(20) DEFAULT 'active',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);
```

#### 支付订单表 (payments)
```sql
CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id UUID REFERENCES merchants(id),
    order_id VARCHAR(255) NOT NULL,
    amount DECIMAL(36,18) NOT NULL,
    currency VARCHAR(10) NOT NULL,
    blockchain VARCHAR(20) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending',
    payment_address VARCHAR(255),
    transaction_hash VARCHAR(255),
    confirmations INTEGER DEFAULT 0,
    expires_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW(),
    UNIQUE(merchant_id, order_id)
);
```

#### 区块链交易表 (blockchain_transactions)
```sql
CREATE TABLE blockchain_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id UUID REFERENCES payments(id),
    blockchain VARCHAR(20) NOT NULL,
    transaction_hash VARCHAR(255) UNIQUE NOT NULL,
    from_address VARCHAR(255) NOT NULL,
    to_address VARCHAR(255) NOT NULL,
    amount DECIMAL(36,18) NOT NULL,
    gas_fee DECIMAL(36,18),
    block_number BIGINT,
    confirmations INTEGER DEFAULT 0,
    status VARCHAR(20) DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT NOW()
);
```

### 4.2 索引策略
```sql
-- 支付订单索引
CREATE INDEX idx_payments_merchant_order ON payments(merchant_id, order_id);
CREATE INDEX idx_payments_status ON payments(status);
CREATE INDEX idx_payments_created_at ON payments(created_at);

-- 区块链交易索引
CREATE INDEX idx_blockchain_tx_hash ON blockchain_transactions(transaction_hash);
CREATE INDEX idx_blockchain_payment_id ON blockchain_transactions(payment_id);
CREATE INDEX idx_blockchain_status ON blockchain_transactions(status);
```

## 5. API设计

### 5.1 第三方集成API

#### 创建支付订单
```http
POST /api/v1/payments
Authorization: Bearer {api_key}
Content-Type: application/json

{
    "order_id": "ORDER_123456",
    "amount": "100.50",
    "currency": "USDT",
    "blockchain": "ethereum",
    "callback_url": "https://merchant.com/webhook",
    "expires_in": 3600
}
```

#### 查询支付状态
```http
GET /api/v1/payments/{payment_id}
Authorization: Bearer {api_key}
```

#### Webhook回调格式
```json
{
    "event": "payment.completed",
    "payment_id": "uuid",
    "order_id": "ORDER_123456",
    "amount": "100.50",
    "currency": "USDT",
    "status": "completed",
    "transaction_hash": "0x...",
    "confirmations": 12,
    "timestamp": "2024-01-01T12:00:00Z",
    "signature": "hmac_sha256_signature"
}
```

### 5.2 管理后台API
- 商户管理: CRUD操作
- 支付统计: 交易量、成功率分析
- 财务报表: 收入、手续费统计
- 系统监控: 服务状态、性能指标

## 6. 安全设计

### 6.1 认证授权
- **API密钥**: 商户身份验证
- **HMAC签名**: 请求完整性验证
- **JWT Token**: 管理后台会话管理
- **IP白名单**: 限制访问来源

### 6.2 资金安全
- **多重签名**: 热钱包资金保护
- **冷热分离**: 大额资金冷存储
- **风控系统**: 异常交易监控
- **资金托管**: 第三方托管服务

### 6.3 数据安全
- **数据加密**: 敏感信息AES加密
- **传输加密**: HTTPS/TLS通信
- **访问控制**: RBAC权限模型
- **审计日志**: 操作记录追踪

## 7. 监控与运维

### 7.1 监控指标
- **业务指标**: 支付成功率、平均处理时间
- **技术指标**: CPU、内存、磁盘使用率
- **区块链指标**: 节点连接状态、区块同步进度

### 7.2 告警策略
- **支付失败率** > 5%
- **API响应时间** > 2秒
- **区块链节点离线**
- **资金池余额不足**

### 7.3 日志管理
- **结构化日志**: JSON格式
- **日志等级**: ERROR、WARN、INFO、DEBUG
- **日志聚合**: ELK Stack
- **日志保留**: 90天

## 8. 部署架构

### 8.1 开发环境
```yaml
# docker-compose.dev.yml
version: '3.8'
services:
  postgres:
    image: postgres:15
    environment:
      POSTGRES_DB: wopay_dev
      POSTGRES_USER: wopay
      POSTGRES_PASSWORD: password
    ports:
      - "5432:5432"
  
  redis:
    image: redis:7
    ports:
      - "6379:6379"
  
  rabbitmq:
    image: rabbitmq:3-management
    ports:
      - "5672:5672"
      - "15672:15672"
```

### 8.2 生产环境
- **负载均衡**: Nginx + 多实例部署
- **数据库**: PostgreSQL主从复制
- **缓存**: Redis集群
- **消息队列**: RabbitMQ集群
- **监控**: Prometheus + Grafana

## 9. 扩展性设计

### 9.1 水平扩展
- **无状态服务**: 所有业务服务无状态设计
- **数据库分片**: 按商户ID分片
- **缓存策略**: Redis集群 + 一致性哈希

### 9.2 新链集成
- **适配器模式**: 统一区块链接口
- **插件化设计**: 新链作为插件加载
- **配置驱动**: 链参数配置化管理

这个架构设计为WoPay提供了完整的技术蓝图，支持高并发、高可用的Web3支付服务。
