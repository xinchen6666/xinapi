package controller

import (
	"context"
	"errors"
	"mime"
	"net/http"
	"strings"
	"time"
	"unicode"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/dto"
	"github.com/QuantumNous/new-api/model"
	"github.com/QuantumNous/new-api/service"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
)

const (
	accountEgressInvalidCode     = "account_egress_request_invalid"
	accountEgressUnavailableCode = "account_egress_unavailable"
)

func invalidAccountEgressRequest(c *gin.Context) {
	common.ApiErrorStatus(c, http.StatusBadRequest, common.NewMessage("Invalid parameters"), gin.H{"code": accountEgressInvalidCode})
}

func decodeAccountEgressCommand(c *gin.Context, dst any, limit int64) bool {
	mediaType, _, err := mime.ParseMediaType(c.GetHeader("Content-Type"))
	if err != nil || mediaType != "application/json" {
		invalidAccountEgressRequest(c)
		return false
	}
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, limit)
	if err := common.DecodeJsonBody(c.Request.Body, dst); err != nil {
		invalidAccountEgressRequest(c)
		return false
	}
	return true
}

func validAccountEgressAccountID(id string) bool {
	return len(id) > len("acct_") && len(id) <= 128 && strings.HasPrefix(id, "acct_") &&
		!strings.ContainsFunc(id, func(r rune) bool { return unicode.IsSpace(r) || unicode.IsControl(r) })
}

func canonicalAccountEgressUUID(value *string) bool {
	id, err := uuid.Parse(*value)
	if err != nil || id == uuid.Nil {
		return false
	}
	*value = id.String()
	return true
}

func validAccountEgressGroupIDs(groups []string) bool {
	if len(groups) > 1000 {
		return false
	}
	seen := make(map[string]struct{}, len(groups))
	for _, group := range groups {
		suffix, ok := strings.CutPrefix(group, "grp_")
		if !ok || len(suffix) != 32 {
			return false
		}
		for i := range len(suffix) {
			b := suffix[i]
			if (b < '0' || b > '9') && (b < 'a' || b > 'f') {
				return false
			}
		}
		if _, exists := seen[group]; exists {
			return false
		}
		seen[group] = struct{}{}
	}
	return true
}

func accountEgressResponse(c *gin.Context, data any, err error) {
	if err == nil {
		common.ApiSuccess(c, data)
		return
	}
	var exposed *service.CPRManagementError
	if errors.As(err, &exposed) {
		common.ApiErrorStatus(c, exposed.Status, common.NewMessage(exposed.Message), gin.H{"code": exposed.Code})
		return
	}
	common.ApiErrorStatus(c, http.StatusServiceUnavailable, common.NewMessage("Account and egress service is unavailable"), gin.H{"code": accountEgressUnavailableCode})
}

// accountEgressAudit stores approved identifiers and an explicit result, including
// business failures whose transport returned 200. The framework fallback stays
// active for requests rejected before a command passes validation.
func accountEgressAudit(c *gin.Context, action, entityType, entityID, operationID, result, code string) {
	params := map[string]any{"entity_type": entityType, "result": result}
	if entityID != "" {
		params["entity_id"] = entityID
	}
	if operationID != "" {
		params["operation_id"] = operationID
	}
	if code != "" {
		params["code"] = code
	}
	markAuditLogged(c)
	ctx, cancel := context.WithTimeout(context.WithoutCancel(c.Request.Context()), 5*time.Second)
	defer cancel()
	auditContext := c.Copy()
	auditContext.Request = c.Request.WithContext(ctx)
	info := &model.AuditRequestInfo{
		Method: c.Request.Method, Route: c.FullPath(), Path: c.FullPath(),
		Status: c.Writer.Status(), Success: result == "success",
	}
	model.RecordOperationAuditLog(c.GetInt("id"), c.GetInt("role"), action, c.ClientIP(), action, params, auditOperatorInfo(c), info, auditContext)
}

func accountEgressErrorOutcome(err error) (string, string) {
	var exposed *service.CPRManagementError
	if errors.As(err, &exposed) {
		if exposed.ResultUnknown {
			return "unknown", exposed.Code
		}
		return "failure", exposed.Code
	}
	var sourceError *service.SubscriptionSourceError
	if errors.As(err, &sourceError) {
		return "failure", sourceError.Code
	}
	var previewError *service.SubscriptionPreviewError
	if errors.As(err, &previewError) {
		return "failure", previewError.Code
	}
	return "failure", accountEgressUnavailableCode
}

func finishAccountEgressAction(c *gin.Context, data any, err error, action, entityType, entityID, operationID, result string) {
	var code string
	if err != nil {
		result, code = accountEgressErrorOutcome(err)
	}
	accountEgressResponse(c, data, err)
	accountEgressAudit(c, action, entityType, entityID, operationID, result, code)
}

func GetAccountEgressDetail(c *gin.Context) {
	id := c.Query("account_id")
	if !validAccountEgressAccountID(id) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := service.ReadAccountEgressDetail(c.Request.Context(), id)
	accountEgressResponse(c, data, err)
}

func GetAccountEgressGroupOptions(c *gin.Context) {
	page, pageSize, valid := accountEgressPagination(c)
	if !valid {
		return
	}
	data, err := service.ReadAccountEgressGroupOptions(c.Request.Context(), page, pageSize)
	accountEgressResponse(c, data, err)
}

func UpdateAccountEgressSettings(c *gin.Context) {
	var command dto.AccountEgressSettings
	if !decodeAccountEgressCommand(c, &command, 96<<10) {
		return
	}
	if !validAccountEgressAccountID(command.AccountID) || (command.Enabled == nil && command.GroupIDs == nil) ||
		(command.GroupIDs == nil) != (command.ExpectedGroupIDs == nil) ||
		(command.GroupIDs != nil && (!validAccountEgressGroupIDs(*command.GroupIDs) || !validAccountEgressGroupIDs(*command.ExpectedGroupIDs))) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := service.UpdateAccountEgressSettings(c.Request.Context(), command)
	var unavailable *service.CPRManagementError
	if errors.As(err, &unavailable) && unavailable.Code == "account_settings_readback_unavailable" {
		accountEgressResponse(c, nil, err)
		accountEgressAudit(c, "account_egress.account_settings", "account", command.AccountID, "", "success", unavailable.Code)
		return
	}
	finishAccountEgressAction(c, data, err, "account_egress.account_settings", "account", command.AccountID, "", "success")
}

func RefreshAccountEgressQuota(c *gin.Context) {
	var command dto.AccountEgressAccountID
	if !decodeAccountEgressCommand(c, &command, 4096) {
		return
	}
	if !validAccountEgressAccountID(command.AccountID) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := service.RefreshAccountEgressQuota(c.Request.Context(), command.AccountID)
	finishAccountEgressAction(c, data, err, "account_egress.quota_refresh", "account", command.AccountID, "", "success")
}

func GetAccountEgressEngines(c *gin.Context) {
	data, err := service.ReadAccountEgressEngines(c.Request.Context())
	accountEgressResponse(c, data, err)
}

func StopAccountEgressEngine(c *gin.Context) {
	var command dto.AccountEgressEntryID
	if !decodeAccountEgressCommand(c, &command, 4096) {
		return
	}
	if len(command.EntryID) == 0 || len(command.EntryID) > 256 || strings.ContainsFunc(command.EntryID, func(r rune) bool {
		return !(r >= 'a' && r <= 'z' || r >= 'A' && r <= 'Z' || r >= '0' && r <= '9' || r == '-' || r == '_')
	}) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := service.StopAccountEgressEngine(c.Request.Context(), command)
	result := "unknown"
	if data.Engine != nil && data.Engine.State == "stopped" {
		result = "success"
	}
	finishAccountEgressAction(c, data, err, "account_egress.engine_stop", "engine", command.EntryID, "", result)
}

func ReclaimAccountEgressEngine(c *gin.Context) {
	var command dto.AccountEgressOperationID
	if !decodeAccountEgressCommand(c, &command, 4096) {
		return
	}
	if !canonicalAccountEgressUUID(&command.OperationID) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := service.ReclaimAccountEgressEngine(c.Request.Context(), command)
	finishAccountEgressAction(c, data, err, "account_egress.engine_reclaim", "authorization_reservation", command.OperationID, command.OperationID, "success")
}

func TestAccountEgressNode(c *gin.Context) {
	var command dto.AccountEgressTestNode
	if !decodeAccountEgressCommand(c, &command, 4096) {
		return
	}
	if !canonicalAccountEgressUUID(&command.SourceID) || !canonicalAccountEgressUUID(&command.NodeID) || !canonicalAccountEgressUUID(&command.VersionID) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := service.TestAccountEgressNode(c.Request.Context(), command)
	result := "failure"
	if data.Success {
		result = "success"
	}
	finishAccountEgressAction(c, data, err, "account_egress.node_test", "subscription_node", command.NodeID, "", result)
}

func GetAccountEgressManagedRoute(c *gin.Context) {
	id := c.Query("account_id")
	if !validAccountEgressAccountID(id) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := service.ReadAccountEgressManagedRoute(c.Request.Context(), id)
	accountEgressResponse(c, data, err)
}

func GetAccountEgressRouteOperation(c *gin.Context) {
	id := c.Query("operation_id")
	if !canonicalAccountEgressUUID(&id) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := service.ReadAccountEgressRouteOperation(c.Request.Context(), id)
	accountEgressResponse(c, data, err)
}

func PrepareAccountEgressManagedRoute(c *gin.Context) {
	var command dto.AccountEgressPrepareRoute
	if !decodeAccountEgressCommand(c, &command, 4096) {
		return
	}
	if !validAccountEgressAccountID(command.AccountID) || command.ExpectedGeneration == nil || *command.ExpectedGeneration > maxSourceRevision ||
		!canonicalAccountEgressUUID(&command.OperationID) || !canonicalAccountEgressUUID(&command.SourceID) || !canonicalAccountEgressUUID(&command.NodeID) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := service.PrepareAccountEgressManagedRoute(c.Request.Context(), command)
	finishAccountEgressRouteAction(c, "prepare", command.OperationID, &command, data, err)
}

func ApproveAccountEgressManagedRoute(c *gin.Context) {
	accountEgressRouteAction(c, "approve", service.ApproveAccountEgressManagedRoute)
}

func AbortAccountEgressManagedRoute(c *gin.Context) {
	accountEgressRouteAction(c, "abort", service.AbortAccountEgressManagedRoute)
}

func ReconcileAccountEgressManagedRoute(c *gin.Context) {
	accountEgressRouteAction(c, "reconcile", service.ReconcileAccountEgressManagedRoute)
}

func accountEgressRouteAction(c *gin.Context, action string, send func(context.Context, dto.AccountEgressOperationID) (dto.AccountEgressRouteReceipt, error)) {
	var command dto.AccountEgressOperationID
	if !decodeAccountEgressCommand(c, &command, 4096) {
		return
	}
	if !canonicalAccountEgressUUID(&command.OperationID) {
		invalidAccountEgressRequest(c)
		return
	}
	data, err := send(c.Request.Context(), command)
	finishAccountEgressRouteAction(c, action, command.OperationID, nil, data, err)
}

func finishAccountEgressRouteAction(c *gin.Context, action, operationID string, prepare *dto.AccountEgressPrepareRoute, data dto.AccountEgressRouteReceipt, err error) {
	entityID := ""
	if prepare != nil {
		entityID = prepare.AccountID
	}
	var exposed *service.CPRManagementError
	if errors.As(err, &exposed) && exposed.ResultUnknown {
		// Only the receipt is read back. The original command remains single-use.
		readback, readErr := service.ReadAccountEgressRouteOperation(context.WithoutCancel(c.Request.Context()), operationID)
		if readErr == nil && (prepare == nil || accountEgressPrepareMatches(readback.Operation, *prepare)) {
			if accountEgressRouteOutcome(action, readback) != "unknown" {
				data, err = readback, nil
			}
		}
	}
	if data.Operation != nil {
		entityID = data.Operation.AccountID
	}
	finishAccountEgressAction(c, data, err, "account_egress.route_"+action, "account_route", entityID, operationID, accountEgressRouteOutcome(action, data))
}

func accountEgressPrepareMatches(operation *dto.AccountEgressRouteOperation, input dto.AccountEgressPrepareRoute) bool {
	return operation != nil && operation.AccountID == input.AccountID && operation.ExpectedGeneration == *input.ExpectedGeneration &&
		operation.Candidate.SourceID == input.SourceID && operation.Candidate.NodeID == input.NodeID
}

func accountEgressRouteOutcome(action string, receipt dto.AccountEgressRouteReceipt) string {
	op := receipt.Operation
	if op == nil {
		return "unknown"
	}
	if op.Phase == "failed" {
		if op.ErrorCode != nil && *op.ErrorCode == "route_restore_required" {
			return "unknown"
		}
		if action == "abort" && op.ErrorCode != nil && *op.ErrorCode == "route_operation_cancelled" && !receipt.RequiresReconciliation {
			return "success"
		}
		return "failure"
	}
	if action == "prepare" && op.Phase == "ready" {
		return "success"
	}
	if action != "abort" && op.Phase == "applied" && !receipt.RequiresReconciliation {
		return "success"
	}
	return "unknown"
}
