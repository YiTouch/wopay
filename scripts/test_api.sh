#!/bin/bash
# WoPay API测试脚本
# 用于快速测试API接口功能

BASE_URL="http://localhost:8080"
API_V1="$BASE_URL/api/v1"

echo "🚀 WoPay API测试脚本"
echo "===================="

# 1. 健康检查
echo "1. 检查服务健康状态..."
curl -s "$BASE_URL/health" | jq '.'
echo ""

# 2. 获取系统状态
echo "2. 获取系统状态..."
curl -s "$API_V1/status" | jq '.'
echo ""

# 3. 获取网络状态
echo "3. 获取区块链网络状态..."
curl -s "$API_V1/network/status" | jq '.'
echo ""

# 4. 注册测试商户
echo "4. 注册测试商户..."
MERCHANT_RESPONSE=$(curl -s -X POST "$API_V1/merchants" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "测试商户",
    "email": "test@example.com",
    "webhook_url": "https://webhook.site/unique-id"
  }')

echo "$MERCHANT_RESPONSE" | jq '.'

# 提取API密钥
API_KEY=$(echo "$MERCHANT_RESPONSE" | jq -r '.data.api_key // empty')
MERCHANT_ID=$(echo "$MERCHANT_RESPONSE" | jq -r '.data.merchant_id // empty')

if [ -z "$API_KEY" ] || [ "$API_KEY" = "null" ]; then
  echo "❌ 商户注册失败，无法继续测试"
  exit 1
fi

echo "✅ 商户注册成功，API Key: $API_KEY"
echo ""

# 5. 获取商户信息
echo "5. 获取商户信息..."
curl -s "$API_V1/merchants/$MERCHANT_ID" \
  -H "X-API-Key: $API_KEY" | jq '.'
echo ""

# 6. 创建支付订单
echo "6. 创建支付订单..."
PAYMENT_RESPONSE=$(curl -s -X POST "$API_V1/payments" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "order_id": "TEST_ORDER_001",
    "amount": "99.99",
    "currency": "USDT",
    "expires_in": 3600
  }')

echo "$PAYMENT_RESPONSE" | jq '.'

# 提取支付ID
PAYMENT_ID=$(echo "$PAYMENT_RESPONSE" | jq -r '.data.payment_id // empty')

if [ -z "$PAYMENT_ID" ] || [ "$PAYMENT_ID" = "null" ]; then
  echo "❌ 支付订单创建失败"
else
  echo "✅ 支付订单创建成功，Payment ID: $PAYMENT_ID"
  echo ""

  # 7. 查询支付详情
  echo "7. 查询支付详情..."
  curl -s "$API_V1/payments/$PAYMENT_ID" \
    -H "X-API-Key: $API_KEY" | jq '.'
  echo ""

  # 8. 获取支付列表
  echo "8. 获取支付列表..."
  curl -s "$API_V1/payments?limit=10" \
    -H "X-API-Key: $API_KEY" | jq '.'
  echo ""
fi

# 9. 获取商户统计
echo "9. 获取商户统计..."
curl -s "$API_V1/merchants/$MERCHANT_ID/stats" \
  -H "X-API-Key: $API_KEY" | jq '.'
echo ""

# 10. 测试Webhook
echo "10. 测试Webhook..."
curl -s -X POST "$API_V1/webhooks/test" \
  -H "Content-Type: application/json" \
  -H "X-API-Key: $API_KEY" \
  -d '{
    "event_type": "payment.completed",
    "test_data": {"message": "测试消息"}
  }' | jq '.'
echo ""

# 11. 获取Webhook统计
echo "11. 获取Webhook统计..."
curl -s "$API_V1/webhooks/stats?days=7" \
  -H "X-API-Key: $API_KEY" | jq '.'
echo ""

echo "🎉 API测试完成！"
echo ""
echo "📝 测试结果摘要:"
echo "- 商户ID: $MERCHANT_ID"
echo "- API密钥: $API_KEY"
echo "- 支付订单ID: $PAYMENT_ID"
echo ""
echo "💡 提示: 请保存API密钥用于后续测试"
