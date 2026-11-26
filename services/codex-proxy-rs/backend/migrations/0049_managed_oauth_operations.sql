-- 授权归属、冻结路由与一次完成领取共享持久操作
create table managed_oauth_operations (
    operation_id uuid primary key,
    request jsonb not null,
    owner jsonb not null,
    planned_account_id text not null,
    is_new boolean not null,
    provider_kind text not null check(provider_kind in ('openai','xai')),
    node jsonb not null,
    observation jsonb,
    expected_source_revision bigint not null check(expected_source_revision>0),
    proxy_id text not null,
    expected_proxy_revision bigint,
    route_generation bigint not null check(route_generation>=0),
    endpoint_url text,
    phase text not null check(phase in ('preparing','pending','completing','completed','failed','cancelled')),
    flow_id text,
    authorization_url text,
    expires_at timestamptz not null,
    receipt_flow_digest text,
    receipt_owner_digest text,
    reason text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now()
);
create unique index managed_oauth_new_account_unique on managed_oauth_operations(planned_account_id) where is_new;
create index managed_oauth_expiry_idx on managed_oauth_operations(expires_at);
