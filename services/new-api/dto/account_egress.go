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

type AccountEgressProxy struct {
	ID                string `json:"id"`
	Name              string `json:"name"`
	Enabled           bool   `json:"enabled"`
	AccountCount      int    `json:"accountCount"`
	HasAuthentication bool   `json:"hasAuthentication"`
}
