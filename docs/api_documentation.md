# WoPay API Documentation

WoPay是一个Web3支付系统，支持以太坊区块链支付。本文档描述了所有可用的API接口。

## 基础信息

- **Base URL**: `https://api.wopay.com/api/v1`
- **认证方式**: API Key (Header: `X-API-Key`)
- **数据格式**: JSON
- **字符编码**: UTF-8

## 认证

大部分API接口需要API密钥认证。在请求头中包含：

```
X-API-Key: your_api_key_here
```

## 响应格式

所有API响应都遵循统一格式：

```json
{
  "success": true,
  "data": { ... },
  "error": null,
  "timestamp": "2024-01-01T00:00:00Z"
}
```

错误响应：
```json
{
  "success": false,
  "data": null,
  "error": "Error message",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

## 商户管理

### 注册商户

创建新的商户账户并获取API密钥。

**请求**
```http
POST /api/v1/merchants
Content-Type: application/json

{
  "name": "My Store",
  "email": "store@example.com",
  "webhook_url": "https://mystore.com/webhook"
}
```

**响应**
```json
{
  "success": true,
  "data": {
    "merchant_id": "123e4567-e89b-12d3-a456-426614174000",
    "name": "My Store",
    "email": "store@example.com",
    "api_key": "wopay_live_1234567890abcdef",
    "api_secret": "wopay_secret_abcdef1234567890",
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

### 获取商户信息

**请求**
```http
GET /api/v1/merchants/{merchant_id}
X-API-Key: your_api_key
```

**响应**
```json
{
  "success": true,
  "data": {
    "id": "123e4567-e89b-12d3-a456-426614174000",
    "name": "My Store",
    "email": "store@example.com",
    "webhook_url": "https://mystore.com/webhook",
    "status": "active",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
  }
}
```

### 更新商户信息

**请求**
```http
PUT /api/v1/merchants/{merchant_id}
X-API-Key: your_api_key
Content-Type: application/json

{
  "name": "Updated Store Name",
  "webhook_url": "https://newdomain.com/webhook"
}
```

### 重新生成API密钥

**请求**
```http
POST /api/v1/merchants/{merchant_id}/regenerate-keys
X-API-Key: your_api_key
```

**响应**
```json
{
  "success": true,
  "data": {
    "api_key": "wopay_live_new1234567890abcdef",
    "api_secret": "wopay_secret_newabcdef1234567890",
    "generated_at": "2024-01-01T00:00:00Z"
  }
}
```

### 获取商户统计

**请求**
```http
GET /api/v1/merchants/{merchant_id}/stats
X-API-Key: your_api_key
```

**响应**
```json
{
  "success": true,
  "data": {
    "total_payments": 150,
    "completed_payments": 142,
    "pending_payments": 5,
    "failed_payments": 3,
    "total_volume": "1250.50",
    "success_rate": 94.67
  }
}
```

## 支付管理

### 创建支付订单

**请求**
```http
POST /api/v1/payments
X-API-Key: your_api_key
Content-Type: application/json

{
  "order_id": "ORDER_20240101_001",
  "amount": "99.99",
  "currency": "USDT",
  "callback_url": "https://mystore.com/payment-callback",
  "expires_in": 3600
}
```

**响应**
```json
{
  "success": true,
  "data": {
    "payment_id": "456e7890-e89b-12d3-a456-426614174000",
    "payment_address": "0x1234567890abcdef1234567890abcdef12345678",
    "amount": "99.99",
    "currency": "USDT",
    "expires_at": "2024-01-01T01:00:00Z",
    "qr_code": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...",
    "payment_url": "ethereum:0x1234567890abcdef1234567890abcdef12345678@1/transfer?address=0x1234567890abcdef1234567890abcdef12345678&uint256=99990000"
  }
}
```

### 查询支付订单

**请求**
```http
GET /api/v1/payments/{payment_id}
X-API-Key: your_api_key
```

**响应**
```json
{
  "success": true,
  "data": {
    "payment_id": "456e7890-e89b-12d3-a456-426614174000",
    "order_id": "ORDER_20240101_001",
    "amount": "99.99",
    "currency": "USDT",
    "payment_address": "0x1234567890abcdef1234567890abcdef12345678",
    "status": "completed",
    "transaction_hash": "0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
    "confirmations": 15,
    "expires_at": "2024-01-01T01:00:00Z",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:05:00Z"
  }
}
```

### 获取支付订单列表

**请求**
```http
GET /api/v1/payments?status=completed&currency=USDT&page=1&limit=20
X-API-Key: your_api_key
```

**响应**
```json
{
  "success": true,
  "data": {
    "payments": [
      {
        "payment_id": "456e7890-e89b-12d3-a456-426614174000",
        "order_id": "ORDER_20240101_001",
        "amount": "99.99",
        "currency": "USDT",
        "status": "completed",
        "created_at": "2024-01-01T00:00:00Z"
      }
    ],
    "pagination": {
      "page": 1,
      "limit": 20,
      "total": 150,
      "pages": 8
    }
  }
}
```

### 获取支付二维码

**请求**
```http
GET /api/v1/payments/{payment_id}/qrcode
X-API-Key: your_api_key
```

**响应**: PNG图片数据

## Webhook

### 测试Webhook

**请求**
```http
POST /api/v1/webhooks/test
X-API-Key: your_api_key
Content-Type: application/json

{
  "event_type": "payment.completed",
  "test_data": {
    "message": "This is a test webhook"
  }
}
```

### 获取Webhook统计

**请求**
```http
GET /api/v1/webhooks/stats?days=30
X-API-Key: your_api_key
```

**响应**
```json
{
  "success": true,
  "data": {
    "total_webhooks": 500,
    "successful_webhooks": 485,
    "failed_webhooks": 10,
    "pending_webhooks": 5,
    "success_rate": 97.0,
    "average_attempts": 1.2
  }
}
```

## 系统状态

### 健康检查

**请求**
```http
GET /health
```

**响应**
```json
{
  "status": "healthy",
  "version": "1.0.0",
  "database": "connected",
  "blockchain": "connected",
  "timestamp": "2024-01-01T00:00:00Z"
}
```

### 系统状态

**请求**
```http
GET /api/v1/status
```

### 区块链网络状态

**请求**
```http
GET /api/v1/network/status
```

**响应**
```json
{
  "success": true,
  "data": {
    "ethereum": {
      "chain_id": 1,
      "block_number": 18500000,
      "gas_price": 25000000000,
      "is_syncing": false,
      "confirmation_blocks": 12
    }
  }
}
```

## Webhook通知

当支付状态发生变化时，系统会向商户配置的Webhook URL发送通知。

### 通知格式

**Headers**
```
Content-Type: application/json
X-WoPay-Signature: sha256=signature_hash
X-WoPay-Webhook-Id: webhook_uuid
```

**Body**
```json
{
  "event_type": "payment_status_changed",
  "timestamp": "2024-01-01T00:05:00Z",
  "data": {
    "payment_id": "456e7890-e89b-12d3-a456-426614174000",
    "order_id": "ORDER_20240101_001",
    "status": "completed",
    "amount": "99.99",
    "currency": "USDT",
    "transaction_hash": "0xabcdef...",
    "confirmations": 15
  }
}
```

### 签名验证

使用HMAC-SHA256验证Webhook签名：

```javascript
const crypto = require('crypto');

function verifyWebhookSignature(payload, signature, secret) {
  const expectedSignature = 'sha256=' + crypto
    .createHmac('sha256', secret)
    .update(payload)
    .digest('hex');
  
  return crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expectedSignature)
  );
}
```

## 错误代码

| 状态码 | 说明 |
|--------|------|
| 200 | 成功 |
| 201 | 创建成功 |
| 400 | 请求参数错误 |
| 401 | 未授权 |
| 403 | 禁止访问 |
| 404 | 资源不存在 |
| 429 | 请求频率限制 |
| 500 | 服务器内部错误 |
| 503 | 服务不可用 |

## 支持的币种

| 币种 | 符号 | 网络 | 合约地址 |
|------|------|------|----------|
| 以太坊 | ETH | Ethereum | - |
| USDT | USDT | Ethereum | 0xdAC17F958D2ee523a2206206994597C13D831ec7 |

## 限流规则

- API接口: 10请求/秒
- Webhook接口: 5请求/秒
- 每个IP每小时最多1000次请求

## 支付状态说明

| 状态 | 说明 |
|------|------|
| pending | 等待支付 |
| confirmed | 已确认 (区块链上已记录，但确认数不足) |
| completed | 已完成 (达到所需确认数) |
| failed | 支付失败 |
| expired | 已过期 |
