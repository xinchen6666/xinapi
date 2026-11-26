-- 订阅来源的公开索引与完整认证加密文档
CREATE TABLE subscription_sources (
    id UUID PRIMARY KEY,
    revision BIGINT NOT NULL CHECK (revision > 0),
    name TEXT NOT NULL,
    document BYTEA NOT NULL CHECK (octet_length(document) <= 33554432),
    updated_at TIMESTAMPTZ NOT NULL
);
