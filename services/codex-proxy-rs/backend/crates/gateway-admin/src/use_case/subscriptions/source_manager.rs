//! 订阅来源刷新、结构化身份匹配与批准版本管理

use super::DefaultSubscriptionsService;
use crate::{
    model::{
        AdminError, MutationContext, subscription_sources::*, subscriptions::SubscriptionPreview,
    },
    ports::{
        store::{AdminStoreError, AdminStoreErrorKind},
        subscription_sources::SubscriptionStore,
    },
};
use chrono::Utc;
use std::collections::{BTreeMap, BTreeSet};

fn store_error(error: AdminStoreError) -> AdminError {
    use crate::model::AdminErrorKind;
    let (kind, message) = match error.kind() {
        AdminStoreErrorKind::StaleRevision
        | AdminStoreErrorKind::Conflict
        | AdminStoreErrorKind::DuplicateName => (
            AdminErrorKind::Conflict,
            "Subscription source changed. Refresh its details before approving.",
        ),
        AdminStoreErrorKind::NotFound => (
            AdminErrorKind::NotFound,
            "Subscription source was not found.",
        ),
        AdminStoreErrorKind::Invalid => (
            AdminErrorKind::Invalid,
            "The subscription source exceeds its storage limits.",
        ),
        AdminStoreErrorKind::Unavailable => (
            AdminErrorKind::Unavailable,
            "Subscription source storage is unavailable.",
        ),
    };
    AdminError::new(kind, message).with_source(error)
}
fn invalid() -> AdminError {
    AdminError::invalid("Review the subscription source parameters and selected scope.")
}
fn conflict() -> AdminError {
    AdminError::conflict("Subscription source changed. Refresh its details before approving.")
}
fn next_revision(value: u64) -> Result<u64, AdminError> {
    value
        .checked_add(1)
        .filter(|v| *v <= i64::MAX as u64)
        .ok_or_else(invalid)
}
fn name(value: &str) -> Result<String, AdminError> {
    let text = value.trim();
    if text.is_empty() || text.chars().count() > 100 || text.chars().any(char::is_control) {
        return Err(invalid());
    }
    Ok(text.to_owned())
}
impl DefaultSubscriptionsService {
    fn persistence(&self) -> Result<&dyn SubscriptionStore, AdminError> {
        self.store
            .as_deref()
            .ok_or_else(|| AdminError::unavailable("Subscription source storage is unavailable."))
    }
    async fn save_source(
        &self,
        document: SourceDocument,
        expected: Option<u64>,
        operation: &'static str,
        context: &MutationContext,
    ) -> Result<SourceDetail, AdminError> {
        let detail = document.detail();
        serde_json::to_writer(
            ResponseBudget(crate::model::subscriptions::MAX_PREVIEW_RESPONSE_BYTES - 128),
            &detail,
        )
        .map_err(|_| AdminError::invalid("The subscription source exceeds its storage limits."))?;
        self.persistence()?
            .save(document, expected, operation, context)
            .await
            .map_err(store_error)?;
        Ok(detail)
    }
    pub(super) async fn source_list(&self) -> Result<Vec<SourceSummary>, AdminError> {
        self.persistence()?.list().await.map_err(store_error)
    }
    pub(super) async fn source_detail(&self, id: &str) -> Result<SourceDetail, AdminError> {
        Ok(self.load_source(id, None).await?.detail())
    }
    async fn load_source(
        &self,
        id: &str,
        expected: Option<u64>,
    ) -> Result<SourceDocument, AdminError> {
        uuid::Uuid::parse_str(id).map_err(|_| invalid())?;
        let document = self.persistence()?.load(id).await.map_err(store_error)?;
        if expected.is_some_and(|v| v != document.summary.revision) {
            return Err(conflict());
        }
        Ok(document)
    }
    pub(super) async fn source_create(
        &self,
        title: &str,
        url: &str,
        context: &MutationContext,
    ) -> Result<SourceDetail, AdminError> {
        let title = name(title)?;
        let url = self.policy.resolve_source(url).map_err(|_| invalid())?;
        let now = Utc::now();
        let document = SourceDocument {
            summary: SourceSummary {
                id: uuid::Uuid::new_v4().to_string(),
                name: title,
                revision: 1,
                approved_version: 0,
                status: "pending".to_owned(),
                last_error_code: None,
                node_count: 0,
                has_pending_review: false,
                updated_at: now,
            },
            source_url: url.to_string(),
            nodes: Vec::new(),
            versions: Vec::new(),
            pending: None,
            approved_sets: Vec::new(),
        };
        self.save_source(document, None, "create", context).await
    }
    pub(super) async fn source_update(
        &self,
        id: &str,
        expected: u64,
        title: &str,
        url: Option<&str>,
        context: &MutationContext,
    ) -> Result<SourceDetail, AdminError> {
        let mut document = self.load_source(id, Some(expected)).await?;
        document.summary.name = name(title)?;
        if let Some(url) = url {
            document.source_url = self
                .policy
                .resolve_source(url)
                .map_err(|_| invalid())?
                .to_string();
        }
        document.pending = None;
        document.summary.revision = next_revision(expected)?;
        document.summary.has_pending_review = false;
        document.summary.status = "source_updated".into();
        document.summary.last_error_code = None;
        document.summary.updated_at = Utc::now();
        self.save_source(document, Some(expected), "update", context)
            .await
    }
    pub(super) async fn source_refresh(
        &self,
        id: &str,
        expected: u64,
        context: &MutationContext,
    ) -> Result<SourceDetail, AdminError> {
        use super::SubscriptionsService as _;
        let mut document = self.load_source(id, Some(expected)).await?;
        document.summary.revision = next_revision(expected)?;
        document.summary.updated_at = Utc::now();
        document.pending = None;
        let operation = match self.preview(&document.source_url).await {
            Ok(preview) => {
                let pending = prepare_review(&document, preview)?;
                document.pending = Some(pending);
                document.summary.status = "awaiting_approval".into();
                document.summary.last_error_code = None;
                document.summary.has_pending_review = true;
                "review"
            }
            Err(error) => {
                document.summary.status = "stale".into();
                document.summary.last_error_code = Some(error.reason().into());
                document.summary.has_pending_review = false;
                "refresh_failed"
            }
        };
        // CAS 令下载期间发生的编辑或批准优先保留，旧结果只返回冲突
        self.save_source(document, Some(expected), operation, context)
            .await
    }
    pub(super) async fn source_approve(
        &self,
        command: ApproveSource,
        context: &MutationContext,
    ) -> Result<SourceDetail, AdminError> {
        let mut document = self
            .load_source(&command.source_id, Some(command.expected_revision))
            .await?;
        let pending = document.pending.clone().ok_or_else(conflict)?;
        if pending.view.id != command.review_id
            || pending.view.expected_revision != command.expected_revision
        {
            return Err(conflict());
        }
        if command.selected.len() > 4096 {
            return Err(invalid());
        }
        if (!pending.view.complete
            || command.selected.len()
                != pending
                    .view
                    .candidates
                    .iter()
                    .filter(|v| v.parse_valid)
                    .count())
            && !command.accept_partial
        {
            return Err(invalid());
        }
        let mut selected_indices = BTreeSet::new();
        let mut selected_ids = BTreeSet::new();
        let mut approved = Vec::new();
        let mut versions = Vec::new();
        for selected in &command.selected {
            if !selected_indices.insert(selected.candidate_index) {
                return Err(invalid());
            }
            let candidate = pending
                .view
                .candidates
                .get(selected.candidate_index)
                .ok_or_else(invalid)?;
            let connection = pending
                .connections
                .get(selected.candidate_index)
                .and_then(Option::as_ref)
                .ok_or_else(invalid)?;
            if !candidate.parse_valid {
                return Err(invalid());
            }
            let id = match &selected.node_id {
                Some(id) => {
                    if !candidate.previous_node_ids.contains(id) {
                        return Err(invalid());
                    }
                    id.clone()
                }
                None => uuid::Uuid::new_v4().to_string(),
            };
            if !selected_ids.insert(id.clone()) {
                return Err(invalid());
            }
            let previous = document.nodes.iter().find(|n| n.id == id);
            let current = previous
                .and_then(|node| document.versions.iter().find(|v| v.id == node.version_id));
            let version_id = if let Some(version) = current.filter(|v| &v.connection == connection)
            {
                version.id.clone()
            } else {
                let version = ConnectionVersion {
                    id: uuid::Uuid::new_v4().to_string(),
                    node_id: id.clone(),
                    connection: connection.clone(),
                    created_at: Utc::now(),
                };
                let result = version.id.clone();
                versions.push(version);
                result
            };
            approved.push(StoredNode {
                id,
                name: candidate.name.clone(),
                protocol: candidate.protocol.clone(),
                version_id,
                available: true,
                supported: candidate.supported,
            });
        }
        let removed = document
            .nodes
            .iter()
            .filter(|v| v.available && !selected_ids.contains(&v.id))
            .count();
        if pending.view.complete && removed > 0 && !command.confirm_removals {
            return Err(invalid());
        }
        let mut nodes = document.nodes.clone();
        if pending.view.complete {
            for node in &mut nodes {
                node.available = false;
            }
        }
        for selected in approved {
            if let Some(node) = nodes.iter_mut().find(|v| v.id == selected.id) {
                *node = selected;
            } else {
                nodes.push(selected);
            }
        }
        if nodes.len() > 4096 {
            return Err(AdminError::invalid(
                "The subscription source exceeds its storage limits.",
            ));
        }
        let changed = document.nodes != nodes;
        document.nodes = nodes;
        document.versions.extend(versions);
        if changed || document.approved_sets.is_empty() {
            document.summary.approved_version = next_revision(document.summary.approved_version)?;
            document.approved_sets.push(ApprovedSet {
                id: uuid::Uuid::new_v4().to_string(),
                version: document.summary.approved_version,
                created_at: Utc::now(),
                nodes: document.nodes.clone(),
            });
        }
        document.summary.revision = next_revision(command.expected_revision)?;
        document.summary.node_count = document.nodes.iter().filter(|n| n.available).count();
        document.summary.status = if pending.view.complete {
            "approved"
        } else {
            "approved_partial"
        }
        .into();
        document.summary.last_error_code = None;
        document.summary.has_pending_review = false;
        document.summary.updated_at = Utc::now();
        document.pending = None;
        self.save_source(
            document,
            Some(command.expected_revision),
            "approve",
            context,
        )
        .await
    }
}

fn prepare_review(
    document: &SourceDocument,
    preview: SubscriptionPreview,
) -> Result<PendingReview, AdminError> {
    let versions = document
        .versions
        .iter()
        .map(|v| (v.id.as_str(), &v.connection))
        .collect::<BTreeMap<_, _>>();
    let complete = preview
        .diagnostics
        .iter()
        .all(|d| d.code.as_str() == "unsupported_protocol")
        && preview
            .candidates
            .iter()
            .all(|n| n.parse_valid && n.connection.is_some());
    let connections = preview
        .candidates
        .iter()
        .map(|v| v.connection.clone())
        .collect::<Vec<_>>();
    let mut reference_count = 0usize;
    let candidates = preview
        .candidates
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let same = document
                .nodes
                .iter()
                .filter(|old| {
                    node.connection.as_ref().is_some_and(|connection| {
                        versions
                            .get(old.version_id.as_str())
                            .is_some_and(|v| *v == connection)
                    })
                })
                .collect::<Vec<_>>();
            let duplicate = node
                .connection
                .as_ref()
                .is_some_and(|c| connections.iter().filter(|v| v.as_ref() == Some(c)).count() > 1);
            let by_name = document
                .nodes
                .iter()
                .filter(|old| old.name == node.name)
                .collect::<Vec<_>>();
            let suggested = if same.len() == 1 && !duplicate {
                Some(same[0].id.clone())
            } else {
                None
            };
            let previous = if same.is_empty() {
                by_name
            } else {
                same.clone()
            };
            let change = if !node.parse_valid {
                "invalid"
            } else if duplicate || same.len() > 1 {
                "ambiguous"
            } else if let Some(id) = &suggested {
                if document
                    .nodes
                    .iter()
                    .any(|old| &old.id == id && old.name == node.name)
                {
                    "unchanged"
                } else {
                    "renamed"
                }
            } else if previous.is_empty() {
                "added"
            } else {
                "connection_changed"
            };
            // 映射关系先计数后复制，重复连接的审核图始终处于公开响应预算内
            reference_count += previous.len();
            if reference_count > 32768 {
                return Err(AdminError::invalid(
                    "The subscription source exceeds its storage limits.",
                ));
            }
            Ok(ReviewCandidate {
                index,
                name: node.name.clone(),
                protocol: node.protocol.as_str().into(),
                parse_valid: node.parse_valid,
                supported: node.supported,
                change: change.into(),
                suggested_node_id: suggested,
                previous_node_ids: previous.iter().map(|v| v.id.clone()).collect(),
            })
        })
        .collect::<Result<Vec<_>, AdminError>>()?;
    let missing_node_ids = document
        .nodes
        .iter()
        .filter(|old| {
            old.available
                && !candidates
                    .iter()
                    .any(|c| c.previous_node_ids.contains(&old.id))
        })
        .map(|v| v.id.clone())
        .collect();
    Ok(PendingReview {
        view: ReviewView {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: document.summary.id.clone(),
            expected_revision: document.summary.revision,
            complete,
            candidates,
            missing_node_ids,
            diagnostics: preview
                .diagnostics
                .iter()
                .map(|d| ReviewDiagnostic {
                    code: d.code.as_str().into(),
                    message: d.message.clone(),
                    subject: d.subject.clone(),
                })
                .collect(),
            created_at: Utc::now(),
        },
        connections,
    })
}

/// 流式计量 JSON 编码，只保存计数并在容量耗尽时结束编码
struct ResponseBudget(usize);
impl std::io::Write for ResponseBudget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0 = self
            .0
            .checked_sub(bytes.len())
            .ok_or_else(|| std::io::Error::other("preview capacity"))?;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
