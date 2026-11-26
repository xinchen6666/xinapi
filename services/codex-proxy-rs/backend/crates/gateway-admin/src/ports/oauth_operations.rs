//! 授权操作的持久领取、冻结条件与只读完成事实

use super::store::AdminStoreResult;
use crate::model::{
    MutationActor, oauth_operations::OAuthOperation, provider_credentials::AuthorizationStarted,
};
use async_trait::async_trait;

#[async_trait]
pub trait OAuthOperationStore: Send + Sync {
    async fn owns_flow(&self, flow_id: &str) -> AdminStoreResult<bool>;
    async fn begin(&self, operation: OAuthOperation) -> AdminStoreResult<(OAuthOperation, bool)>;
    async fn get(
        &self,
        operation_id: &str,
        owner: &MutationActor,
    ) -> AdminStoreResult<Option<OAuthOperation>>;
    async fn endpoint(
        &self,
        operation_id: &str,
        owner: &MutationActor,
        endpoint: &str,
    ) -> AdminStoreResult<()>;
    async fn ready(
        &self,
        operation_id: &str,
        owner: &MutationActor,
        started: AuthorizationStarted,
        observation: crate::model::managed_routes::RouteObservation,
    ) -> AdminStoreResult<OAuthOperation>;
    async fn claim(
        &self,
        operation_id: &str,
        owner: &MutationActor,
    ) -> AdminStoreResult<Option<OAuthOperation>>;
    async fn fail(
        &self,
        operation_id: &str,
        owner: &MutationActor,
        reason: &str,
    ) -> AdminStoreResult<()>;
    async fn cancel(
        &self,
        operation_id: &str,
        owner: &MutationActor,
    ) -> AdminStoreResult<OAuthOperation>;
    async fn reclaimable(
        &self,
        operation_id: &str,
        owner: &MutationActor,
    ) -> AdminStoreResult<bool>;
}
