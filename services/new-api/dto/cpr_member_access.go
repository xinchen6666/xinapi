package dto

// CprMemberAccess 是管理员可见的成员授权视图。
type CprMemberAccess struct {
	UserID      int      `json:"user_id"`
	Revision    int64    `json:"revision"`
	GroupIDs    []string `json:"group_ids"`
	Status      string   `json:"status"`
	Stage       string   `json:"stage,omitempty"`
	OperationID string   `json:"operation_id,omitempty"`
	CprKeyID    string   `json:"cpr_key_id,omitempty"`
	UpdatedAt   int64    `json:"updated_at,omitempty"`
}

type CprMemberGroupsCommand struct {
	GroupIDs         []string `json:"group_ids"`
	ExpectedRevision *int64   `json:"expected_revision"`
}

type CprMemberReconcileCommand struct {
	OperationID      string `json:"operation_id"`
	ExpectedRevision *int64 `json:"expected_revision"`
}
