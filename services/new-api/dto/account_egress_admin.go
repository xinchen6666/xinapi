package dto

import (
	"net/netip"
	"time"
)

// AccountEgressDetail limits the account data sent to the browser to management display fields.
type AccountEgressDetail struct {
	Account *AccountEgressAccountDetail `json:"account"`
}

type AccountEgressAccountDetail struct {
	AccountEgressAccount
	Groups       []AccountEgressGroup      `json:"groups"`
	Capabilities AccountEgressCapabilities `json:"capabilities"`
	Capacity     AccountEgressAccountSlots `json:"capacity"`
	Quota        AccountEgressQuota        `json:"quota"`
}

type AccountEgressGroup struct {
	ID      string `json:"id"`
	Name    string `json:"name"`
	Color   string `json:"color"`
	Enabled bool   `json:"enabled"`
}

type AccountEgressCapabilities struct {
	Quota        bool `json:"quota"`
	QuotaRefresh bool `json:"quotaRefresh"`
}

type AccountEgressAccountSlots struct {
	UsedSlots  *uint64 `json:"usedSlots"`
	TotalSlots *uint64 `json:"totalSlots"`
}

type AccountEgressQuota struct {
	PausedWindows    []string                   `json:"pausedWindows"`
	RefreshedAt      *time.Time                 `json:"refreshedAt"`
	LimitReached     bool                       `json:"limitReached"`
	RateLimitedUntil *time.Time                 `json:"rateLimitedUntil"`
	Windows          []AccountEgressQuotaWindow `json:"windows"`
	Credits          *AccountEgressQuotaCredits `json:"credits"`
}

type AccountEgressQuotaWindow struct {
	Key            string   `json:"key"`
	Group          string   `json:"group"`
	Role           *string  `json:"role"`
	WindowSeconds  *uint64  `json:"windowSeconds"`
	UsedPercent    *float64 `json:"usedPercent"`
	LimitReached   bool     `json:"limitReached"`
	Stale          bool     `json:"stale"`
	LabelDisplay   string   `json:"labelDisplay"`
	ResetAtDisplay string   `json:"resetAtDisplay"`
}

type AccountEgressQuotaCredits struct {
	HasCredits bool    `json:"hasCredits"`
	Unlimited  bool    `json:"unlimited"`
	Balance    *string `json:"balance"`
}

type AccountEgressSettings struct {
	AccountID        string    `json:"account_id"`
	Enabled          *bool     `json:"enabled,omitempty"`
	GroupIDs         *[]string `json:"group_ids,omitempty"`
	ExpectedGroupIDs *[]string `json:"expected_group_ids,omitempty"`
}

type AccountEgressAccountID struct {
	AccountID string `json:"account_id"`
}

type AccountEgressEntryID struct {
	EntryID string `json:"entry_id"`
}

type AccountEgressOperationID struct {
	OperationID string `json:"operation_id"`
}

type AccountEgressTestNode struct {
	SourceID  string `json:"source_id"`
	NodeID    string `json:"node_id"`
	VersionID string `json:"version_id"`
}

type AccountEgressPrepareRoute struct {
	OperationID        string  `json:"operation_id"`
	AccountID          string  `json:"account_id"`
	ExpectedGeneration *uint64 `json:"expected_generation"`
	SourceID           string  `json:"source_id"`
	NodeID             string  `json:"node_id"`
}

type AccountEgressEngineList struct {
	Items    []AccountEgressEngine     `json:"items"`
	Capacity *AccountEgressEngineSlots `json:"capacity"`
}

type AccountEgressEngineSlots struct {
	Limit     uint64 `json:"limit"`
	Used      uint64 `json:"used"`
	Available uint64 `json:"available"`
}

type AccountEgressEngineResult struct {
	Engine *AccountEgressEngine `json:"engine"`
}

type AccountEgressReclaimed struct {
	OperationID string `json:"operation_id"`
	Status      string `json:"status"`
}

type AccountEgressEngine struct {
	EntryID      string                    `json:"entry_id"`
	Owner        *AccountEgressEngineOwner `json:"owner"`
	SourceID     string                    `json:"source_id"`
	NodeID       string                    `json:"node_id"`
	VersionID    string                    `json:"version_id"`
	NodeName     string                    `json:"node_name"`
	Protocol     string                    `json:"protocol"`
	State        string                    `json:"state"`
	StartedAt    *time.Time                `json:"started_at"`
	RSSBytes     *uint64                   `json:"rss_bytes"`
	CPUSeconds   *float64                  `json:"cpu_seconds"`
	ColdStartMS  *uint64                   `json:"cold_start_ms"`
	LastError    *string                   `json:"last_error"`
	OccupiesSlot bool                      `json:"occupies_slot"`
	Stoppable    bool                      `json:"stoppable"`
	ExpiresAt    *time.Time                `json:"expires_at"`
	Reclaimable  bool                      `json:"reclaimable"`
}

type AccountEgressEngineOwner struct {
	Kind string `json:"kind"`
	ID   string `json:"id"`
}

type AccountEgressObservation struct {
	Success   bool        `json:"success"`
	LatencyMS uint64      `json:"latency_ms"`
	ExitIP    *netip.Addr `json:"exit_ip,omitempty"`
	ExitIPv4  *netip.Addr `json:"exit_ipv4,omitempty"`
	ExitIPv6  *netip.Addr `json:"exit_ipv6,omitempty"`
}

type AccountEgressRouteNode struct {
	SourceID  string `json:"source_id"`
	NodeID    string `json:"node_id"`
	VersionID string `json:"version_id"`
	NodeName  string `json:"node_name"`
	Protocol  string `json:"protocol"`
}

type AccountEgressManagedRoute struct {
	RouteID             string                    `json:"route_id"`
	AccountID           string                    `json:"account_id"`
	ProxyID             *string                   `json:"proxy_id"`
	AccountProxyID      *string                   `json:"account_proxy_id"`
	ProxyEnabled        *bool                     `json:"proxy_enabled"`
	Generation          uint64                    `json:"generation"`
	Current             *AccountEgressRouteNode   `json:"current"`
	Observation         *AccountEgressObservation `json:"observation"`
	ApplyingOperationID *string                   `json:"applying_operation_id"`
	AppliedOperationID  *string                   `json:"applied_operation_id"`
	UpdatedAt           time.Time                 `json:"updated_at"`
}

type AccountEgressRouteOperation struct {
	OperationID        string                    `json:"operation_id"`
	AccountID          string                    `json:"account_id"`
	RouteID            string                    `json:"route_id"`
	ExpectedGeneration uint64                    `json:"expected_generation"`
	ExpectedProxyID    *string                   `json:"expected_proxy_id"`
	Candidate          AccountEgressRouteNode    `json:"candidate"`
	Phase              string                    `json:"phase"`
	Observation        *AccountEgressObservation `json:"observation"`
	ErrorCode          *string                   `json:"error_code"`
	CreatedAt          time.Time                 `json:"created_at"`
	UpdatedAt          time.Time                 `json:"updated_at"`
}

type AccountEgressRouteReceipt struct {
	Operation              *AccountEgressRouteOperation `json:"operation"`
	Route                  *AccountEgressManagedRoute   `json:"route"`
	Engine                 *AccountEgressEngine         `json:"engine"`
	RequiresReconciliation bool                         `json:"requires_reconciliation"`
}
