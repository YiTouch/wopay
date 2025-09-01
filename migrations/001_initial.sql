-- WoPay MVP 数据库初始化脚本
-- 创建时间: 2025-01-01
-- 描述: 创建商户、支付订单和区块链交易表

-- 启用UUID扩展
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- 商户表
CREATE TABLE merchants (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    api_key VARCHAR(255) UNIQUE NOT NULL,
    api_secret VARCHAR(255) NOT NULL,
    webhook_url VARCHAR(500),
    status VARCHAR(20) DEFAULT 'active' CHECK (status IN ('active', 'inactive', 'suspended')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 支付订单表
CREATE TABLE payments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    merchant_id UUID NOT NULL REFERENCES merchants(id) ON DELETE CASCADE,
    order_id VARCHAR(255) NOT NULL,
    amount DECIMAL(36,18) NOT NULL CHECK (amount > 0),
    currency VARCHAR(10) NOT NULL CHECK (currency IN ('ETH', 'USDT')),
    payment_address VARCHAR(42) NOT NULL,
    status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed', 'completed', 'expired', 'failed')),
    transaction_hash VARCHAR(66),
    confirmations INTEGER DEFAULT 0 CHECK (confirmations >= 0),
    expires_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    UNIQUE(merchant_id, order_id)
);

-- 区块链交易表
CREATE TABLE blockchain_transactions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    payment_id UUID NOT NULL REFERENCES payments(id) ON DELETE CASCADE,
    blockchain VARCHAR(20) NOT NULL DEFAULT 'ethereum',
    transaction_hash VARCHAR(66) UNIQUE NOT NULL,
    from_address VARCHAR(42) NOT NULL,
    to_address VARCHAR(42) NOT NULL,
    amount DECIMAL(36,18) NOT NULL CHECK (amount > 0),
    gas_fee DECIMAL(36,18),
    block_number BIGINT,
    confirmations INTEGER DEFAULT 0 CHECK (confirmations >= 0),
    status VARCHAR(20) DEFAULT 'pending' CHECK (status IN ('pending', 'confirmed', 'failed')),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Webhook日志表
CREATE TABLE webhook_logs (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    payment_id UUID NOT NULL REFERENCES payments(id) ON DELETE CASCADE,
    webhook_url VARCHAR(500) NOT NULL,
    payload JSONB NOT NULL,
    response_status INTEGER,
    response_body TEXT,
    retry_count INTEGER DEFAULT 0,
    success BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 创建索引
CREATE INDEX idx_payments_merchant_order ON payments(merchant_id, order_id);
CREATE INDEX idx_payments_status ON payments(status);
CREATE INDEX idx_payments_created_at ON payments(created_at);
CREATE INDEX idx_payments_expires_at ON payments(expires_at);

CREATE INDEX idx_blockchain_tx_hash ON blockchain_transactions(transaction_hash);
CREATE INDEX idx_blockchain_payment_id ON blockchain_transactions(payment_id);
CREATE INDEX idx_blockchain_status ON blockchain_transactions(status);
CREATE INDEX idx_blockchain_block_number ON blockchain_transactions(block_number);

CREATE INDEX idx_webhook_logs_payment_id ON webhook_logs(payment_id);
CREATE INDEX idx_webhook_logs_success ON webhook_logs(success);
CREATE INDEX idx_webhook_logs_created_at ON webhook_logs(created_at);

-- 创建更新时间触发器函数
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- 为需要的表创建更新时间触发器
CREATE TRIGGER update_merchants_updated_at 
    BEFORE UPDATE ON merchants 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_payments_updated_at 
    BEFORE UPDATE ON payments 
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- 插入测试数据
INSERT INTO merchants (name, email, api_key, api_secret, webhook_url) VALUES 
('测试商户', 'test@example.com', 'test_api_key_123456', 'test_api_secret_789012', 'https://webhook.site/test');

-- 添加注释
COMMENT ON TABLE merchants IS '商户信息表';
COMMENT ON TABLE payments IS '支付订单表';
COMMENT ON TABLE blockchain_transactions IS '区块链交易记录表';
COMMENT ON TABLE webhook_logs IS 'Webhook回调日志表';

COMMENT ON COLUMN merchants.api_key IS 'API访问密钥';
COMMENT ON COLUMN merchants.api_secret IS 'API签名密钥';
COMMENT ON COLUMN payments.payment_address IS '收款地址';
COMMENT ON COLUMN payments.confirmations IS '区块确认数';
COMMENT ON COLUMN blockchain_transactions.gas_fee IS 'Gas费用';
COMMENT ON COLUMN webhook_logs.retry_count IS '重试次数';
