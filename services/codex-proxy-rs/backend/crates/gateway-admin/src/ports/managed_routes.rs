//! 受管路由与账号代理发布的持久化端口

use crate::{
    model::{
        MutationContext, Revision,
        managed_routes::{BeginRoute, ManagedRoute, RouteObservation, RouteOperation},
    },
    ports::store::AdminStoreResult,
};
use async_trait::async_trait;
use gateway_core::account::OutboundProxy;

#[async_trait]
pub trait ManagedRouteStore: Send + Sync {
    /// 相同operation复用既有回执，bool表示本次创建
    async fn begin(
        &self,
        command: BeginRoute,
        context: &MutationContext,
    ) -> AdminStoreResult<(RouteOperation, bool)>;
    async fn ready(
        &self,
        operation_id: &str,
        observation: RouteObservation,
    ) -> AdminStoreResult<RouteOperation>;
    async fn claim(&self, operation_id: &str) -> AdminStoreResult<RouteOperation>;
    /// 引擎状态已核对后一次提交代理、账号绑定、出口代次及回执
    async fn publish(
        &self,
        operation_id: &str,
        endpoint: OutboundProxy,
        context: &MutationContext,
    ) -> AdminStoreResult<(RouteOperation, Revision)>;
    async fn fail(&self, operation_id: &str, reason: &str) -> AdminStoreResult<RouteOperation>;
    async fn operation(&self, operation_id: &str) -> AdminStoreResult<RouteOperation>;
    async fn route(&self, account_id: &str) -> AdminStoreResult<Option<ManagedRoute>>;
}
