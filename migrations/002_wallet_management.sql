-- 钱包管理相关表结构

-- 支付地址表
CREATE TABLE payment_addresses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    payment_id UUID NOT NULL REFERENCES payments(id),
    address_index INTEGER NOT NULL,
    address VARCHAR(42) NOT NULL UNIQUE,
    private_key_encrypted TEXT NOT NULL,
    is_collected BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 归集交易表
CREATE TABLE collection_transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    from_address VARCHAR(42) NOT NULL,
    to_address VARCHAR(42) NOT NULL,
    amount DECIMAL(36,18) NOT NULL,
    tx_hash VARCHAR(66) NOT NULL UNIQUE,
    gas_used BIGINT,
    gas_price VARCHAR(32),
    status VARCHAR(20) DEFAULT 'pending',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 钱包配置表
CREATE TABLE wallet_config (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    master_address VARCHAR(42) NOT NULL,
    collection_threshold DECIMAL(36,18) DEFAULT 0.1,
    auto_collection_enabled BOOLEAN DEFAULT TRUE,
    collection_interval_minutes INTEGER DEFAULT 60,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- 创建索引
CREATE INDEX idx_payment_addresses_payment_id ON payment_addresses(payment_id);
CREATE INDEX idx_payment_addresses_address ON payment_addresses(address);
CREATE INDEX idx_payment_addresses_is_collected ON payment_addresses(is_collected);
CREATE INDEX idx_collection_transactions_from_address ON collection_transactions(from_address);
CREATE INDEX idx_collection_transactions_tx_hash ON collection_transactions(tx_hash);

-- 插入默认配置
INSERT INTO wallet_config (master_address, collection_threshold) 
VALUES ('0x0000000000000000000000000000000000000000', 0.1);
