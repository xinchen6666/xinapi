package dto

// SubscriptionSourceSummary 保留来源展示元数据，链接由提交请求传递。
type SubscriptionSourceSummary struct {
	ID               string  `json:"id"`
	Name             string  `json:"name"`
	Revision         uint64  `json:"revision"`
	ApprovedVersion  uint64  `json:"approved_version"`
	Status           string  `json:"status"`
	LastErrorCode    *string `json:"last_error_code,omitempty"`
	NodeCount        int     `json:"node_count"`
	HasPendingReview bool    `json:"has_pending_review"`
	UpdatedAt        string  `json:"updated_at"`
}
type SubscriptionSourceList struct {
	Items []SubscriptionSourceSummary `json:"items"`
}
type SubscriptionSourceNode struct {
	ID        string `json:"id"`
	Name      string `json:"name"`
	Protocol  string `json:"protocol"`
	VersionID string `json:"version_id"`
	Available bool   `json:"available"`
	Supported bool   `json:"supported"`
}
type SubscriptionSourceCandidate struct {
	Index           int      `json:"index"`
	Name            string   `json:"name"`
	Protocol        string   `json:"protocol"`
	ParseValid      bool     `json:"parse_valid"`
	Supported       bool     `json:"supported"`
	Change          string   `json:"change"`
	SuggestedNodeID *string  `json:"suggested_node_id,omitempty"`
	PreviousNodeIDs []string `json:"previous_node_ids"`
}
type SubscriptionSourceReview struct {
	ID               string                                `json:"id"`
	SourceID         string                                `json:"source_id"`
	ExpectedRevision uint64                                `json:"expected_revision"`
	Complete         bool                                  `json:"complete"`
	Candidates       []SubscriptionSourceCandidate         `json:"candidates"`
	MissingNodeIDs   []string                              `json:"missing_node_ids"`
	Diagnostics      []AccountEgressSubscriptionDiagnostic `json:"diagnostics"`
	CreatedAt        string                                `json:"created_at"`
}
type SubscriptionSourceDetail struct {
	Source SubscriptionSourceSummary `json:"source"`
	Nodes  []SubscriptionSourceNode  `json:"nodes"`
	Review *SubscriptionSourceReview `json:"review,omitempty"`
}
type SubscriptionSourceCreate struct {
	Name      string `json:"name"`
	SourceURL string `json:"source_url"`
}
type SubscriptionSourceUpdate struct {
	SourceID         string  `json:"source_id"`
	ExpectedRevision uint64  `json:"expected_revision"`
	Name             string  `json:"name"`
	SourceURL        *string `json:"source_url,omitempty"`
}
type SubscriptionSourceRefresh struct {
	SourceID         string `json:"source_id"`
	ExpectedRevision uint64 `json:"expected_revision"`
}
type SubscriptionSourceSelection struct {
	CandidateIndex int     `json:"candidate_index"`
	NodeID         *string `json:"node_id"`
}
type SubscriptionSourceApprove struct {
	SourceID         string                        `json:"source_id"`
	ExpectedRevision uint64                        `json:"expected_revision"`
	ReviewID         string                        `json:"review_id"`
	Selected         []SubscriptionSourceSelection `json:"selected"`
	AcceptPartial    bool                          `json:"accept_partial"`
	ConfirmRemovals  bool                          `json:"confirm_removals"`
}
