package dto

// AccountEgressPage 只保留管理列表的分页和展示字段。
type AccountEgressPage[T any] struct {
	Items []T                   `json:"items"`
	Page  AccountEgressPageMeta `json:"page"`
}

type AccountEgressPageMeta struct {
	Page       int `json:"page"`
	PageSize   int `json:"pageSize"`
	Total      int `json:"total"`
	TotalPages int `json:"totalPages"`
}

type AccountEgressAccount struct {
	ID              string  `json:"id"`
	Name            string  `json:"name"`
	Provider        string  `json:"provider"`
	Status          string  `json:"status"`
	Enabled         bool    `json:"enabled"`
	PlanType        *string `json:"planType"`
	OutboundProxyID *string `json:"outboundProxyId"`
}

// AccountEgressSubscriptionPreview 只保留订阅预览的展示字段，CPR 返回的其余字段丢弃。
// 字段名与订阅预览接口的既有合同一致，源码串沿用 snake_case。
type AccountEgressSubscriptionPreview struct {
	SourceURL    string                                `json:"source_url"`
	Format       string                                `json:"format"`
	FetchedBytes int                                   `json:"fetched_bytes"`
	Accepted     int                                   `json:"accepted"`
	Rejected     int                                   `json:"rejected"`
	Candidates   []AccountEgressSubscriptionCandidate  `json:"candidates"`
	Diagnostics  []AccountEgressSubscriptionDiagnostic `json:"diagnostics"`
}

// AccountEgressSubscriptionCandidate 分别描述解析有效性与原生传输能力。
type AccountEgressSubscriptionCandidate struct {
	Name       string `json:"name"`
	Protocol   string `json:"protocol"`
	ParseValid bool   `json:"parse_valid"`
	Supported  bool   `json:"supported"`
}

// AccountEgressSubscriptionDiagnostic 描述逐项解析说明，Subject 指向节点或字段。
type AccountEgressSubscriptionDiagnostic struct {
	Code    string `json:"code"`
	Message string `json:"message"`
	Subject string `json:"subject"`
}

type AccountEgressProxy struct {
	ID                string `json:"id"`
	Name              string `json:"name"`
	Enabled           bool   `json:"enabled"`
	AccountCount      int    `json:"accountCount"`
	HasAuthentication bool   `json:"hasAuthentication"`
}
