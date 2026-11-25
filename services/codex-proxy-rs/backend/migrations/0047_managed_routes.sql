-- 受管路由只描述出口发布事实，账号关联仍由 provider_accounts.outbound_proxy_id 持有
alter table outbound_proxies
    add column route_generation bigint not null default 0 check (route_generation >= 0);

create table managed_routes (
    route_id uuid primary key,
    account_id text not null unique references provider_accounts(id) on delete cascade,
    proxy_id text unique references outbound_proxies(id) on delete restrict,
    generation bigint not null default 0 check (generation >= 0),
    current_node jsonb,
    observation jsonb,
    applying_operation_id uuid,
    updated_at timestamptz not null default now(),
    check ((generation = 0 and current_node is null and proxy_id is null)
        or (generation > 0 and current_node is not null and proxy_id is not null)),
    check (current_node is null or jsonb_typeof(current_node) = 'object'),
    check (observation is null or jsonb_typeof(observation) = 'object')
);

-- 操作历史独立保留，删除账号后仍可按 operation_id 读取原回执
create table managed_route_operations (
    operation_id uuid primary key,
    route_id uuid not null,
    account_id text not null,
    expected_generation bigint not null check (expected_generation >= 0),
    expected_proxy_id text,
    expected_proxy_revision bigint,
    expected_proxy_url text,
    expected_source_revision bigint not null check (expected_source_revision > 0),
    candidate jsonb not null check (jsonb_typeof(candidate) = 'object'),
    phase text not null check (phase in ('preparing','ready','applying','applied','failed')),
    observation jsonb,
    error_code text,
    context jsonb not null check (jsonb_typeof(context) = 'object'),
    published_config_revision bigint check (published_config_revision > 0),
    published_proxy_url text,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    check (expected_proxy_revision is null or expected_proxy_revision > 0),
    check (observation is null or jsonb_typeof(observation) = 'object'),
    check (error_code is null or char_length(error_code) between 1 and 128),
    check ((phase = 'applied') = (published_config_revision is not null)),
    check ((phase = 'applied') = (published_proxy_url is not null))
);
create index managed_route_operations_account_idx
    on managed_route_operations(account_id, created_at desc);
create index managed_route_operations_route_idx
    on managed_route_operations(route_id, created_at desc);
create unique index managed_route_operations_one_applying
    on managed_route_operations(route_id) where phase = 'applying';
