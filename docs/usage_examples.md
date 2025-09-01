# WoPay使用示例

本文档提供了WoPay Web3支付系统的完整使用示例，包括不同编程语言的SDK示例。

## 快速开始

### 1. 商户注册

```bash
curl -X POST https://api.wopay.com/api/v1/merchants \
  -H "Content-Type: application/json" \
  -d '{
    "name": "我的商店",
    "email": "store@example.com",
    "webhook_url": "https://mystore.com/webhook"
  }'
```

响应会包含您的API密钥，请妥善保存。

### 2. 创建支付订单

```bash
curl -X POST https://api.wopay.com/api/v1/payments \
  -H "Content-Type: application/json" \
  -H "X-API-Key: your_api_key" \
  -d '{
    "order_id": "ORDER_001",
    "amount": "99.99",
    "currency": "USDT",
    "expires_in": 3600
  }'
```

### 3. 查询支付状态

```bash
curl -X GET https://api.wopay.com/api/v1/payments/{payment_id} \
  -H "X-API-Key: your_api_key"
```

## JavaScript/Node.js示例

### 安装依赖

```bash
npm install axios crypto
```

### 基础SDK实现

```javascript
const axios = require('axios');
const crypto = require('crypto');

class WoPaySDK {
  constructor(apiKey, apiSecret, baseUrl = 'https://api.wopay.com/api/v1') {
    this.apiKey = apiKey;
    this.apiSecret = apiSecret;
    this.baseUrl = baseUrl;
    
    this.client = axios.create({
      baseURL: baseUrl,
      headers: {
        'Content-Type': 'application/json',
        'X-API-Key': apiKey
      }
    });
  }

  // 创建支付订单
  async createPayment(orderData) {
    try {
      const response = await this.client.post('/payments', orderData);
      return response.data;
    } catch (error) {
      throw new Error(`创建支付失败: ${error.response?.data?.error || error.message}`);
    }
  }

  // 查询支付状态
  async getPayment(paymentId) {
    try {
      const response = await this.client.get(`/payments/${paymentId}`);
      return response.data;
    } catch (error) {
      throw new Error(`查询支付失败: ${error.response?.data?.error || error.message}`);
    }
  }

  // 获取支付列表
  async listPayments(params = {}) {
    try {
      const response = await this.client.get('/payments', { params });
      return response.data;
    } catch (error) {
      throw new Error(`获取支付列表失败: ${error.response?.data?.error || error.message}`);
    }
  }

  // 验证Webhook签名
  verifyWebhookSignature(payload, signature) {
    const expectedSignature = 'sha256=' + crypto
      .createHmac('sha256', this.apiSecret)
      .update(payload)
      .digest('hex');
    
    return crypto.timingSafeEqual(
      Buffer.from(signature),
      Buffer.from(expectedSignature)
    );
  }
}

// 使用示例
const wopay = new WoPaySDK('your_api_key', 'your_api_secret');

// 创建支付
async function createPaymentExample() {
  try {
    const payment = await wopay.createPayment({
      order_id: 'ORDER_' + Date.now(),
      amount: '99.99',
      currency: 'USDT',
      expires_in: 3600
    });
    
    console.log('支付创建成功:', payment.data);
    console.log('支付地址:', payment.data.payment_address);
    console.log('二维码:', payment.data.qr_code);
    
    return payment.data.payment_id;
  } catch (error) {
    console.error('创建支付失败:', error.message);
  }
}

// 查询支付状态
async function checkPaymentStatus(paymentId) {
  try {
    const payment = await wopay.getPayment(paymentId);
    console.log('支付状态:', payment.data.status);
    return payment.data;
  } catch (error) {
    console.error('查询支付失败:', error.message);
  }
}

module.exports = WoPaySDK;
```

### Express.js Webhook处理

```javascript
const express = require('express');
const WoPaySDK = require('./wopay-sdk');

const app = express();
const wopay = new WoPaySDK('your_api_key', 'your_api_secret');

// Webhook接收端点
app.post('/webhook', express.raw({type: 'application/json'}), (req, res) => {
  const signature = req.headers['x-wopay-signature'];
  const payload = req.body.toString();

  // 验证签名
  if (!wopay.verifyWebhookSignature(payload, signature)) {
    return res.status(401).send('Invalid signature');
  }

  const data = JSON.parse(payload);
  
  // 处理不同事件类型
  switch (data.event_type) {
    case 'payment_status_changed':
      handlePaymentStatusChanged(data.data);
      break;
    default:
      console.log('Unknown event type:', data.event_type);
  }

  res.status(200).send('OK');
});

function handlePaymentStatusChanged(paymentData) {
  console.log('支付状态变更:', paymentData);
  
  if (paymentData.status === 'completed') {
    // 支付完成，发货或提供服务
    console.log(`订单 ${paymentData.order_id} 支付完成，金额: ${paymentData.amount} ${paymentData.currency}`);
    // 在这里添加您的业务逻辑
  }
}

app.listen(3000, () => {
  console.log('Webhook服务器运行在端口3000');
});
```

## PHP示例

### 基础SDK实现

```php
<?php

class WoPaySDK {
    private $apiKey;
    private $apiSecret;
    private $baseUrl;

    public function __construct($apiKey, $apiSecret, $baseUrl = 'https://api.wopay.com/api/v1') {
        $this->apiKey = $apiKey;
        $this->apiSecret = $apiSecret;
        $this->baseUrl = $baseUrl;
    }

    // 创建支付订单
    public function createPayment($orderData) {
        $url = $this->baseUrl . '/payments';
        
        $headers = [
            'Content-Type: application/json',
            'X-API-Key: ' . $this->apiKey
        ];

        $ch = curl_init();
        curl_setopt($ch, CURLOPT_URL, $url);
        curl_setopt($ch, CURLOPT_POST, true);
        curl_setopt($ch, CURLOPT_POSTFIELDS, json_encode($orderData));
        curl_setopt($ch, CURLOPT_HTTPHEADER, $headers);
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);

        $response = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);
        curl_close($ch);

        if ($httpCode !== 201) {
            throw new Exception('创建支付失败: ' . $response);
        }

        return json_decode($response, true);
    }

    // 查询支付状态
    public function getPayment($paymentId) {
        $url = $this->baseUrl . '/payments/' . $paymentId;
        
        $headers = [
            'X-API-Key: ' . $this->apiKey
        ];

        $ch = curl_init();
        curl_setopt($ch, CURLOPT_URL, $url);
        curl_setopt($ch, CURLOPT_HTTPHEADER, $headers);
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);

        $response = curl_exec($ch);
        $httpCode = curl_getinfo($ch, CURLINFO_HTTP_CODE);
        curl_close($ch);

        if ($httpCode !== 200) {
            throw new Exception('查询支付失败: ' . $response);
        }

        return json_decode($response, true);
    }

    // 验证Webhook签名
    public function verifyWebhookSignature($payload, $signature) {
        $expectedSignature = 'sha256=' . hash_hmac('sha256', $payload, $this->apiSecret);
        return hash_equals($signature, $expectedSignature);
    }
}

// 使用示例
$wopay = new WoPaySDK('your_api_key', 'your_api_secret');

// 创建支付
try {
    $payment = $wopay->createPayment([
        'order_id' => 'ORDER_' . time(),
        'amount' => '99.99',
        'currency' => 'USDT',
        'expires_in' => 3600
    ]);
    
    echo "支付创建成功: " . $payment['data']['payment_id'] . "\n";
    echo "支付地址: " . $payment['data']['payment_address'] . "\n";
} catch (Exception $e) {
    echo "错误: " . $e->getMessage() . "\n";
}

// Webhook处理
if ($_SERVER['REQUEST_METHOD'] === 'POST') {
    $signature = $_SERVER['HTTP_X_WOPAY_SIGNATURE'] ?? '';
    $payload = file_get_contents('php://input');

    if (!$wopay->verifyWebhookSignature($payload, $signature)) {
        http_response_code(401);
        exit('Invalid signature');
    }

    $data = json_decode($payload, true);
    
    if ($data['event_type'] === 'payment_status_changed') {
        $paymentData = $data['data'];
        
        if ($paymentData['status'] === 'completed') {
            // 支付完成处理逻辑
            echo "订单 {$paymentData['order_id']} 支付完成\n";
        }
    }

    http_response_code(200);
    echo 'OK';
}
?>
```

## Python示例

### 基础SDK实现

```python
import requests
import hmac
import hashlib
import json
from typing import Dict, Any, Optional

class WoPaySDK:
    def __init__(self, api_key: str, api_secret: str, base_url: str = 'https://api.wopay.com/api/v1'):
        self.api_key = api_key
        self.api_secret = api_secret
        self.base_url = base_url
        self.session = requests.Session()
        self.session.headers.update({
            'Content-Type': 'application/json',
            'X-API-Key': api_key
        })

    def create_payment(self, order_data: Dict[str, Any]) -> Dict[str, Any]:
        """创建支付订单"""
        response = self.session.post(f'{self.base_url}/payments', json=order_data)
        response.raise_for_status()
        return response.json()

    def get_payment(self, payment_id: str) -> Dict[str, Any]:
        """查询支付状态"""
        response = self.session.get(f'{self.base_url}/payments/{payment_id}')
        response.raise_for_status()
        return response.json()

    def list_payments(self, params: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """获取支付列表"""
        response = self.session.get(f'{self.base_url}/payments', params=params)
        response.raise_for_status()
        return response.json()

    def verify_webhook_signature(self, payload: str, signature: str) -> bool:
        """验证Webhook签名"""
        expected_signature = 'sha256=' + hmac.new(
            self.api_secret.encode(),
            payload.encode(),
            hashlib.sha256
        ).hexdigest()
        
        return hmac.compare_digest(signature, expected_signature)

# 使用示例
if __name__ == '__main__':
    wopay = WoPaySDK('your_api_key', 'your_api_secret')

    # 创建支付
    try:
        payment = wopay.create_payment({
            'order_id': f'ORDER_{int(time.time())}',
            'amount': '99.99',
            'currency': 'USDT',
            'expires_in': 3600
        })
        
        print(f"支付创建成功: {payment['data']['payment_id']}")
        print(f"支付地址: {payment['data']['payment_address']}")
        
        # 轮询检查支付状态
        payment_id = payment['data']['payment_id']
        while True:
            status = wopay.get_payment(payment_id)
            print(f"支付状态: {status['data']['status']}")
            
            if status['data']['status'] in ['completed', 'failed', 'expired']:
                break
                
            time.sleep(10)  # 10秒后再次检查
            
    except requests.exceptions.RequestException as e:
        print(f"API请求失败: {e}")
```

### Flask Webhook处理

```python
from flask import Flask, request, jsonify
import json

app = Flask(__name__)
wopay = WoPaySDK('your_api_key', 'your_api_secret')

@app.route('/webhook', methods=['POST'])
def webhook_handler():
    signature = request.headers.get('X-WoPay-Signature', '')
    payload = request.get_data(as_text=True)

    # 验证签名
    if not wopay.verify_webhook_signature(payload, signature):
        return jsonify({'error': 'Invalid signature'}), 401

    data = json.loads(payload)
    
    # 处理支付状态变更
    if data['event_type'] == 'payment_status_changed':
        payment_data = data['data']
        
        if payment_data['status'] == 'completed':
            # 支付完成处理
            print(f"订单 {payment_data['order_id']} 支付完成")
            # 在这里添加您的业务逻辑
            
    return jsonify({'status': 'ok'})

if __name__ == '__main__':
    app.run(port=5000)
```

## 完整电商集成示例

### 电商网站支付流程

```javascript
// 前端: 创建支付订单
async function createPaymentOrder(orderInfo) {
  try {
    const response = await fetch('/api/create-payment', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(orderInfo)
    });
    
    const result = await response.json();
    
    if (result.success) {
      // 显示支付二维码
      showPaymentQRCode(result.data);
      // 开始轮询支付状态
      pollPaymentStatus(result.data.payment_id);
    }
  } catch (error) {
    console.error('创建支付失败:', error);
  }
}

// 显示支付二维码
function showPaymentQRCode(paymentData) {
  const qrCodeImg = document.getElementById('qr-code');
  qrCodeImg.src = paymentData.qr_code;
  
  document.getElementById('payment-address').textContent = paymentData.payment_address;
  document.getElementById('payment-amount').textContent = `${paymentData.amount} ${paymentData.currency}`;
}

// 轮询支付状态
async function pollPaymentStatus(paymentId) {
  const maxAttempts = 120; // 最多轮询2小时
  let attempts = 0;

  const poll = async () => {
    if (attempts >= maxAttempts) {
      showPaymentTimeout();
      return;
    }

    try {
      const response = await fetch(`/api/payment-status/${paymentId}`);
      const result = await response.json();

      if (result.data.status === 'completed') {
        showPaymentSuccess(result.data);
      } else if (result.data.status === 'failed') {
        showPaymentFailed(result.data);
      } else if (result.data.status === 'expired') {
        showPaymentExpired(result.data);
      } else {
        // 继续轮询
        attempts++;
        setTimeout(poll, 5000); // 5秒后再次检查
      }
    } catch (error) {
      console.error('查询支付状态失败:', error);
      setTimeout(poll, 10000); // 10秒后重试
    }
  };

  poll();
}
```

### 后端API实现 (Express.js)

```javascript
const express = require('express');
const WoPaySDK = require('./wopay-sdk');

const app = express();
const wopay = new WoPaySDK(process.env.WOPAY_API_KEY, process.env.WOPAY_API_SECRET);

app.use(express.json());

// 创建支付订单
app.post('/api/create-payment', async (req, res) => {
  try {
    const { productId, quantity, customerEmail } = req.body;
    
    // 计算订单金额 (这里简化处理)
    const amount = calculateOrderAmount(productId, quantity);
    
    const payment = await wopay.createPayment({
      order_id: `ORDER_${Date.now()}_${productId}`,
      amount: amount.toString(),
      currency: 'USDT',
      callback_url: 'https://mystore.com/webhook',
      expires_in: 3600
    });

    // 保存订单信息到数据库
    await saveOrderToDatabase({
      payment_id: payment.data.payment_id,
      product_id: productId,
      quantity,
      customer_email: customerEmail,
      amount,
      status: 'pending'
    });

    res.json(payment);
  } catch (error) {
    res.status(400).json({ error: error.message });
  }
});

// 查询支付状态
app.get('/api/payment-status/:paymentId', async (req, res) => {
  try {
    const payment = await wopay.getPayment(req.params.paymentId);
    res.json(payment);
  } catch (error) {
    res.status(400).json({ error: error.message });
  }
});

// Webhook处理
app.post('/webhook', express.raw({type: 'application/json'}), async (req, res) => {
  const signature = req.headers['x-wopay-signature'];
  const payload = req.body.toString();

  if (!wopay.verifyWebhookSignature(payload, signature)) {
    return res.status(401).send('Invalid signature');
  }

  const data = JSON.parse(payload);
  
  if (data.event_type === 'payment_status_changed') {
    const paymentData = data.data;
    
    // 更新订单状态
    await updateOrderStatus(paymentData.payment_id, paymentData.status);
    
    if (paymentData.status === 'completed') {
      // 发送确认邮件
      await sendConfirmationEmail(paymentData.order_id);
      // 触发发货流程
      await triggerShipping(paymentData.order_id);
    }
  }

  res.status(200).send('OK');
});

app.listen(3000, () => {
  console.log('服务器运行在端口3000');
});
```

## 错误处理最佳实践

### 1. 网络错误重试

```javascript
async function apiCallWithRetry(apiCall, maxRetries = 3) {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await apiCall();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      
      // 指数退避
      const delay = Math.pow(2, i) * 1000;
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
}
```

### 2. 支付状态轮询优化

```javascript
class PaymentStatusPoller {
  constructor(wopay, paymentId) {
    this.wopay = wopay;
    this.paymentId = paymentId;
    this.intervals = [5, 10, 15, 30, 60]; // 递增轮询间隔
    this.currentInterval = 0;
  }

  async start(onStatusChange) {
    while (this.currentInterval < this.intervals.length) {
      try {
        const payment = await this.wopay.getPayment(this.paymentId);
        const status = payment.data.status;

        onStatusChange(status, payment.data);

        if (['completed', 'failed', 'expired'].includes(status)) {
          break;
        }

        const delay = this.intervals[this.currentInterval] * 1000;
        await new Promise(resolve => setTimeout(resolve, delay));
        
        if (this.currentInterval < this.intervals.length - 1) {
          this.currentInterval++;
        }
      } catch (error) {
        console.error('轮询支付状态失败:', error);
        await new Promise(resolve => setTimeout(resolve, 10000));
      }
    }
  }
}
```

## 安全建议

### 1. API密钥管理
- 将API密钥存储在环境变量中，不要硬编码
- 定期轮换API密钥
- 使用不同的密钥用于开发和生产环境

### 2. Webhook安全
- 始终验证Webhook签名
- 使用HTTPS接收Webhook
- 实现幂等性处理，防止重复处理

### 3. 错误处理
- 不要在错误消息中暴露敏感信息
- 记录详细的错误日志用于调试
- 实现优雅的降级机制

## 测试环境

### Goerli测试网配置

```bash
# 测试网环境变量
ETHEREUM_RPC_URL=https://eth-goerli.alchemyapi.io/v2/YOUR_API_KEY
ETHEREUM_WS_URL=wss://eth-goerli.alchemyapi.io/v2/YOUR_API_KEY
CHAIN_ID=5
CONFIRMATION_BLOCKS=6
```

### 获取测试代币

1. 访问 [Goerli Faucet](https://goerlifaucet.com/) 获取测试ETH
2. 使用测试USDT合约地址进行测试

## 常见问题

### Q: 支付确认需要多长时间？
A: 主网通常需要12个区块确认（约3-5分钟），测试网需要6个区块确认。

### Q: 如何处理支付超时？
A: 系统会自动标记过期的支付订单。建议设置合理的过期时间（1-24小时）。

### Q: Webhook重试机制是什么？
A: 系统会在5秒、15秒、45秒、135秒、405秒后重试，最多重试5次。

### Q: 支持哪些币种？
A: 目前支持ETH和USDT，后续会添加更多ERC20代币支持。
