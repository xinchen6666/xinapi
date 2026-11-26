package service

import (
	"bytes"
	"context"
	"io"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/dto"
)

const cprAdminOrigin = "http://cpr:8080"

var cprMutationClient = &http.Client{
	Timeout:       90 * time.Second,
	Transport:     &http.Transport{Proxy: nil, MaxIdleConnsPerHost: 2, IdleConnTimeout: 30 * time.Second, ResponseHeaderTimeout: 85 * time.Second},
	CheckRedirect: func(_ *http.Request, _ []*http.Request) error { return http.ErrUseLastResponse },
}

// sendCPRAdmin takes paths from the service's typed operations. The browser's
// headers stay at the public boundary, and a POST body is single-use.
func sendCPRAdmin(ctx context.Context, client *http.Client, method, path string, query url.Values, input any) (*http.Response, error) {
	key, err := readCPRAdminKey()
	if err != nil {
		return nil, err
	}
	var payload []byte
	if input != nil {
		payload, err = common.Marshal(input)
		if err != nil {
			return nil, ErrAccountEgressUnavailable
		}
	}
	address := cprAdminOrigin + path
	if len(query) > 0 {
		address += "?" + query.Encode()
	}
	request, err := http.NewRequestWithContext(ctx, method, address, bytes.NewReader(payload))
	if err != nil {
		return nil, ErrAccountEgressUnavailable
	}
	if method == http.MethodPost {
		request.GetBody = nil
	}
	request.Header.Set("x-api-key", key)
	request.Header.Set("Accept", "application/json")
	if input != nil {
		request.Header.Set("Content-Type", "application/json")
	}
	if requestID, ok := ctx.Value(common.RequestIdKey).(string); ok && requestID != "" {
		request.Header.Set("x-request-id", requestID)
	}
	response, err := client.Do(request)
	if err != nil {
		if method == http.MethodPost {
			return nil, unknownCPRManagementResult()
		}
		return nil, ErrAccountEgressUnavailable
	}
	return response, nil
}

// CPRManagementError carries only statuses and machine codes chosen by the BFF.
type CPRManagementError struct {
	Status        int
	Code          string
	Message       string
	ResultUnknown bool
}

func (e *CPRManagementError) Error() string { return e.Message }

func unknownCPRManagementResult() *CPRManagementError {
	return &CPRManagementError{
		Status: http.StatusBadGateway, Code: "account_egress_result_unknown",
		Message: "The operation result is unknown. Refresh its status before trying again.", ResultUnknown: true,
	}
}

func cprManagementFailure(response *http.Response, method, path string) error {
	var body struct {
		Code   int    `json:"code"`
		Reason string `json:"reason"`
	}
	_ = common.DecodeJsonBody(io.LimitReader(response.Body, 4096), &body)
	status := response.StatusCode
	if method == http.MethodPost && body.Code == 50202 {
		return unknownCPRManagementResult()
	}

	// The stable reason list preserves operator decisions while CPR's message
	// text and diagnostics stay on its side of the boundary.
	message := "The account or egress operation could not be completed."
	code := "account_egress_request_failed"
	switch status {
	case http.StatusBadRequest, http.StatusUnprocessableEntity:
		code = "account_egress_request_invalid"
		message = "Invalid parameters"
	case http.StatusNotFound:
		code = "account_egress_not_found"
		message = "The requested account or egress resource was not found."
	case http.StatusConflict:
		code = "account_egress_conflict"
		message = "Account or egress settings changed. Refresh and review the current state."
		if path == "/api/admin/accounts/batch-update" {
			code = "account_settings_conflict"
		}
	case http.StatusTooManyRequests:
		code = "account_egress_capacity"
		message = "No egress engine slot is available."
	case http.StatusBadGateway:
		if body.Reason != "managed_route_candidate_failed" && method == http.MethodPost {
			return unknownCPRManagementResult()
		}
	case http.StatusGatewayTimeout:
		if body.Reason != "engine_node_test_timeout" && body.Reason != "engine_start_timeout" && method == http.MethodPost {
			return unknownCPRManagementResult()
		}
		message = "The egress engine action timed out."
	default:
		if status >= 500 && method == http.MethodPost && !knownEngineReason(body.Reason) {
			return unknownCPRManagementResult()
		}
		status = http.StatusServiceUnavailable
		code = "account_egress_unavailable"
		message = ErrAccountEgressUnavailable.Error()
	}
	if knownManagementReason(body.Reason) {
		code = body.Reason
	}
	return &CPRManagementError{Status: status, Code: code, Message: message}
}

func knownManagementReason(reason string) bool {
	switch reason {
	case "managed_route_conflict", "managed_route_missing", "managed_route_invalid", "managed_route_capacity",
		"managed_route_candidate_failed", "managed_route_unavailable", "oauth_operation_missing",
		"oauth_operation_conflict", "oauth_operation_unavailable":
		return true
	default:
		return knownEngineReason(reason)
	}
}

func knownEngineReason(reason string) bool {
	switch reason {
	case "engine_binary_missing", "engine_artifact_identity", "engine_capacity_reached", "engine_port_conflict",
		"engine_start_timeout", "engine_cancelled", "engine_shutting_down", "engine_binding_conflict", "engine_entry_unknown",
		"engine_owner_invalid", "engine_registry_unavailable", "engine_host_unavailable", "engine_source_missing",
		"engine_node_missing", "engine_node_inactive", "engine_node_unsupported", "engine_process_failed", "engine_store_unavailable",
		"engine_configuration_invalid", "engine_node_version_changed", "engine_node_test_timeout", "engine_stop_forbidden":
		return true
	default:
		return false
	}
}

func cprManagementData[T any](ctx context.Context, method, path string, query url.Values, input any, timeout time.Duration) (T, error) {
	var result T
	client := cprReadOnlyClient
	if method == http.MethodPost {
		client = cprMutationClient
	}
	ctx, cancel := context.WithTimeout(ctx, timeout)
	defer cancel()
	response, err := sendCPRAdmin(ctx, client, method, path, query, input)
	if err != nil {
		return result, err
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return result, cprManagementFailure(response, method, path)
	}
	var envelope struct {
		Code int `json:"code"`
		Data *T  `json:"data"`
	}
	body := &io.LimitedReader{R: response.Body, N: (2 << 20) + 1}
	if err := common.DecodeJsonBody(body, &envelope); err != nil || body.N == 0 || envelope.Code != http.StatusOK || envelope.Data == nil {
		if method == http.MethodPost {
			return result, unknownCPRManagementResult()
		}
		return result, ErrAccountEgressUnavailable
	}
	return *envelope.Data, nil
}

const (
	cprManagementReadTimeout   = 8 * time.Second
	cprManagementWriteTimeout  = 25 * time.Second
	cprManagementEngineTimeout = 75 * time.Second
	cprManagementQuotaTimeout  = 60 * time.Second
)

// ReadAccountEgressDetail uses the read-only quota view, which has the same
// account data as detail and leaves credential configuration in CPR.
func ReadAccountEgressDetail(ctx context.Context, accountID string) (dto.AccountEgressDetail, error) {
	query := url.Values{"accountId": {accountID}}
	result, err := cprManagementData[dto.AccountEgressDetail](ctx, http.MethodGet, "/api/admin/accounts/quota", query, nil, cprManagementReadTimeout)
	if err != nil {
		return result, err
	}
	if !prepareAccountEgressDetail(&result, accountID) {
		return dto.AccountEgressDetail{}, ErrAccountEgressUnavailable
	}
	return result, nil
}

func ReadAccountEgressGroupOptions(ctx context.Context, page, pageSize int) (dto.AccountEgressPage[dto.AccountEgressGroup], error) {
	query := url.Values{"page": {strconv.Itoa(page)}, "pageSize": {strconv.Itoa(pageSize)}}
	result, err := cprManagementData[dto.AccountEgressPage[dto.AccountEgressGroup]](ctx, http.MethodGet, "/api/admin/account-groups/options", query, nil, cprManagementReadTimeout)
	if result.Items == nil {
		result.Items = []dto.AccountEgressGroup{}
	}
	return result, err
}

func UpdateAccountEgressSettings(ctx context.Context, input dto.AccountEgressSettings) (dto.AccountEgressDetail, error) {
	command := struct {
		AccountIDs       []string  `json:"accountIds"`
		Enabled          *bool     `json:"enabled,omitempty"`
		GroupIDs         *[]string `json:"groupIds,omitempty"`
		ExpectedGroupIDs *[]string `json:"expectedGroupIds,omitempty"`
	}{AccountIDs: []string{input.AccountID}, Enabled: input.Enabled, GroupIDs: input.GroupIDs, ExpectedGroupIDs: input.ExpectedGroupIDs}
	result, err := cprManagementData[struct {
		AccountIDs []string `json:"accountIds"`
	}](ctx, http.MethodPost, "/api/admin/accounts/batch-update", nil, command, cprManagementWriteTimeout)
	if err != nil {
		return dto.AccountEgressDetail{}, err
	}
	if len(result.AccountIDs) != 1 || result.AccountIDs[0] != input.AccountID {
		return dto.AccountEgressDetail{}, unknownCPRManagementResult()
	}
	view, err := ReadAccountEgressDetail(ctx, input.AccountID)
	if err != nil {
		return dto.AccountEgressDetail{}, &CPRManagementError{
			Status: http.StatusServiceUnavailable, Code: "account_settings_readback_unavailable",
			Message: "The account settings were saved. Refresh to view the latest state.",
		}
	}
	return view, nil
}

func RefreshAccountEgressQuota(ctx context.Context, accountID string) (dto.AccountEgressDetail, error) {
	command := struct {
		AccountID string `json:"accountId"`
	}{AccountID: accountID}
	result, err := cprManagementData[dto.AccountEgressDetail](ctx, http.MethodPost, "/api/admin/accounts/quota/refresh", nil, command, cprManagementQuotaTimeout)
	if err != nil {
		return result, err
	}
	if !prepareAccountEgressDetail(&result, accountID) {
		return dto.AccountEgressDetail{}, unknownCPRManagementResult()
	}
	return result, nil
}

func prepareAccountEgressDetail(detail *dto.AccountEgressDetail, accountID string) bool {
	account := detail.Account
	if account == nil || account.ID != accountID {
		return false
	}
	if strings.Contains(account.Name, "@") {
		account.Name = account.ID
	}
	if account.Groups == nil {
		account.Groups = []dto.AccountEgressGroup{}
	}
	if account.Quota.Windows == nil {
		account.Quota.Windows = []dto.AccountEgressQuotaWindow{}
	}
	if account.Quota.PausedWindows == nil {
		account.Quota.PausedWindows = []string{}
	}
	return true
}

func ReadAccountEgressEngines(ctx context.Context) (dto.AccountEgressEngineList, error) {
	result, err := cprManagementData[dto.AccountEgressEngineList](ctx, http.MethodGet, "/api/admin/engines", nil, nil, cprManagementReadTimeout)
	if err != nil {
		return result, err
	}
	if result.Capacity == nil {
		return dto.AccountEgressEngineList{}, ErrAccountEgressUnavailable
	}
	if result.Items == nil {
		result.Items = []dto.AccountEgressEngine{}
	}
	for i := range result.Items {
		sanitizeAccountEgressEngine(&result.Items[i])
	}
	return result, nil
}

func StopAccountEgressEngine(ctx context.Context, input dto.AccountEgressEntryID) (dto.AccountEgressEngineResult, error) {
	result, err := cprManagementData[dto.AccountEgressEngineResult](ctx, http.MethodPost, "/api/admin/engines/stop-account", nil, input, cprManagementWriteTimeout)
	if err != nil {
		return result, err
	}
	if result.Engine == nil || result.Engine.EntryID != input.EntryID {
		return dto.AccountEgressEngineResult{}, unknownCPRManagementResult()
	}
	sanitizeAccountEgressEngine(result.Engine)
	return result, nil
}

func ReclaimAccountEgressEngine(ctx context.Context, input dto.AccountEgressOperationID) (dto.AccountEgressReclaimed, error) {
	result, err := cprManagementData[dto.AccountEgressReclaimed](ctx, http.MethodPost, "/api/admin/engines/reclaim", nil, input, cprManagementWriteTimeout)
	if err != nil {
		return result, err
	}
	if result.OperationID != input.OperationID || result.Status != "cancelled" {
		return dto.AccountEgressReclaimed{}, unknownCPRManagementResult()
	}
	return result, nil
}

func TestAccountEgressNode(ctx context.Context, input dto.AccountEgressTestNode) (dto.AccountEgressObservation, error) {
	// A pointer distinguishes an explicit negative observation from an absent success field.
	result, err := cprManagementData[struct {
		Success *bool `json:"success"`
		dto.AccountEgressObservation
	}](ctx, http.MethodPost, "/api/admin/engines/test-node", nil, input, cprManagementEngineTimeout)
	if err != nil {
		return dto.AccountEgressObservation{}, err
	}
	if result.Success == nil {
		return dto.AccountEgressObservation{}, unknownCPRManagementResult()
	}
	result.AccountEgressObservation.Success = *result.Success
	return result.AccountEgressObservation, nil
}

func sanitizeAccountEgressEngine(engine *dto.AccountEgressEngine) {
	if engine != nil && engine.LastError != nil && !knownEngineReason(*engine.LastError) {
		reason := "engine_process_failed"
		engine.LastError = &reason
	}
}

func ReadAccountEgressManagedRoute(ctx context.Context, accountID string) (dto.AccountEgressManagedRoute, error) {
	query := url.Values{"account_id": {accountID}}
	result, err := cprManagementData[dto.AccountEgressManagedRoute](ctx, http.MethodGet, "/api/admin/managed-routes", query, nil, cprManagementReadTimeout)
	if err == nil && (result.AccountID != accountID || result.RouteID == "") {
		return dto.AccountEgressManagedRoute{}, ErrAccountEgressUnavailable
	}
	return result, err
}

func ReadAccountEgressRouteOperation(ctx context.Context, operationID string) (dto.AccountEgressRouteReceipt, error) {
	query := url.Values{"operation_id": {operationID}}
	result, err := cprManagementData[dto.AccountEgressRouteReceipt](ctx, http.MethodGet, "/api/admin/managed-routes/operations", query, nil, cprManagementReadTimeout)
	if err == nil && !prepareAccountEgressReceipt(&result, operationID) {
		return dto.AccountEgressRouteReceipt{}, ErrAccountEgressUnavailable
	}
	return result, err
}

func PrepareAccountEgressManagedRoute(ctx context.Context, input dto.AccountEgressPrepareRoute) (dto.AccountEgressRouteReceipt, error) {
	return writeCPRAccountEgressRoute(ctx, "/api/admin/managed-routes/prepare", input.OperationID, input)
}

func ApproveAccountEgressManagedRoute(ctx context.Context, input dto.AccountEgressOperationID) (dto.AccountEgressRouteReceipt, error) {
	return writeCPRAccountEgressRoute(ctx, "/api/admin/managed-routes/approve", input.OperationID, input)
}

func AbortAccountEgressManagedRoute(ctx context.Context, input dto.AccountEgressOperationID) (dto.AccountEgressRouteReceipt, error) {
	return writeCPRAccountEgressRoute(ctx, "/api/admin/managed-routes/abort", input.OperationID, input)
}

func ReconcileAccountEgressManagedRoute(ctx context.Context, input dto.AccountEgressOperationID) (dto.AccountEgressRouteReceipt, error) {
	return writeCPRAccountEgressRoute(ctx, "/api/admin/managed-routes/reconcile", input.OperationID, input)
}

func writeCPRAccountEgressRoute(ctx context.Context, path, operationID string, input any) (dto.AccountEgressRouteReceipt, error) {
	result, err := cprManagementData[dto.AccountEgressRouteReceipt](ctx, http.MethodPost, path, nil, input, cprManagementEngineTimeout)
	if err == nil && !prepareAccountEgressReceipt(&result, operationID) {
		return dto.AccountEgressRouteReceipt{}, unknownCPRManagementResult()
	}
	return result, err
}

func prepareAccountEgressReceipt(receipt *dto.AccountEgressRouteReceipt, operationID string) bool {
	operation := receipt.Operation
	if operation == nil || operation.OperationID != operationID {
		return false
	}
	switch operation.Phase {
	case "preparing", "ready", "applying", "applied", "failed":
	default:
		return false
	}
	if operation.ErrorCode != nil {
		switch *operation.ErrorCode {
		case "route_candidate_failed", "route_candidate_superseded", "route_restore_required", "route_prepare_interrupted", "route_operation_cancelled":
		default:
			reason := "managed_route_unavailable"
			operation.ErrorCode = &reason
		}
	}
	sanitizeAccountEgressEngine(receipt.Engine)
	return true
}
