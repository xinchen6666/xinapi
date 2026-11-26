//! 订阅来源、批准版本与刷新审核的私有事实和公开投影

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 连接事实只供服务端匹配与加密持久化，调试输出统一裁剪
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionDocument(pub serde_json::Value);
impl std::fmt::Debug for ConnectionDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConnectionDocument([REDACTED])")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSummary {
    pub id: String,
    pub name: String,
    pub revision: u64,
    pub approved_version: u64,
    pub status: String,
    pub last_error_code: Option<String>,
    pub node_count: usize,
    pub has_pending_review: bool,
    pub updated_at: DateTime<Utc>,
}

/// Store 单边界负责序列化和加密，API 始终使用显式公开投影
#[derive(Clone, Serialize, Deserialize)]
pub struct SourceDocument {
    pub summary: SourceSummary,
    pub source_url: String,
    pub nodes: Vec<StoredNode>,
    pub versions: Vec<ConnectionVersion>,
    pub pending: Option<PendingReview>,
    #[serde(default)]
    pub approved_sets: Vec<ApprovedSet>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedSet {
    pub id: String,
    pub version: u64,
    pub created_at: DateTime<Utc>,
    pub nodes: Vec<StoredNode>,
}
impl std::fmt::Debug for SourceDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceDocument")
            .field("summary", &self.summary)
            .finish_non_exhaustive()
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredNode {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub version_id: String,
    pub available: bool,
    pub supported: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionVersion {
    pub id: String,
    pub node_id: String,
    pub connection: ConnectionDocument,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingReview {
    pub view: ReviewView,
    pub connections: Vec<Option<ConnectionDocument>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewDiagnostic {
    pub code: String,
    pub message: String,
    pub subject: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewCandidate {
    pub index: usize,
    pub name: String,
    pub protocol: String,
    pub parse_valid: bool,
    pub supported: bool,
    pub change: String,
    pub suggested_node_id: Option<String>,
    pub previous_node_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewView {
    pub id: String,
    pub source_id: String,
    pub expected_revision: u64,
    pub complete: bool,
    pub candidates: Vec<ReviewCandidate>,
    pub missing_node_ids: Vec<String>,
    pub diagnostics: Vec<ReviewDiagnostic>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDetail {
    pub source: SourceSummary,
    pub nodes: Vec<StoredNode>,
    pub review: Option<ReviewView>,
}
impl SourceDocument {
    #[must_use]
    pub fn detail(&self) -> SourceDetail {
        SourceDetail {
            source: self.summary.clone(),
            nodes: self.nodes.clone(),
            review: self.pending.as_ref().map(|p| p.view.clone()),
        }
    }
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSelection {
    pub candidate_index: usize,
    pub node_id: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApproveSource {
    pub source_id: String,
    pub expected_revision: u64,
    pub review_id: String,
    pub selected: Vec<NodeSelection>,
    pub accept_partial: bool,
    pub confirm_removals: bool,
}
