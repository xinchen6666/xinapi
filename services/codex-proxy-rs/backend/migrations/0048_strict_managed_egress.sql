-- 已发布受管出口的发送要求独立保存，解绑或代理故障继续保留要求
alter table provider_accounts
    add column managed_egress_required boolean not null default false;

update provider_accounts a
   set managed_egress_required = true
  from managed_routes r
 where r.account_id = a.id
   and r.generation > 0;
