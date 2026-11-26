package controller

import (
	"errors"
	"net/http"
	"strconv"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/dto"
	"github.com/QuantumNous/new-api/middleware"
	"github.com/QuantumNous/new-api/model"
	"github.com/QuantumNous/new-api/service"
	"github.com/QuantumNous/new-api/service/authz"
	"github.com/gin-gonic/gin"
	"gorm.io/gorm"
)

func cprMemberAuthorizer(actorID, actorRole, targetRole int) bool {
	return actorRole >= common.RoleAdminUser && canManageTargetRole(actorRole, targetRole) && authz.Can(actorID, actorRole, authz.AccountEgressManageAccounts)
}

func cprMemberTarget(c *gin.Context) (int, model.AuthSessionIdentity, bool) {
	identity, ok := middleware.GetSessionAuthIdentity(c)
	if !ok {
		cprMemberResponse(c, dto.CprMemberAccess{}, model.ErrUserSessionInactive)
		return 0, model.AuthSessionIdentity{}, false
	}
	session := model.AuthSessionIdentity{UserID: identity.UserID, SessionID: identity.SessionID,
		UserAuthVersion: identity.UserAuthVersion, SessionVersion: identity.SessionVersion}
	userID, err := strconv.Atoi(c.Param("id"))
	if err != nil || userID <= 0 {
		cprMemberResponse(c, dto.CprMemberAccess{}, service.ErrCprMemberAccessInvalid)
		return 0, model.AuthSessionIdentity{}, false
	}
	target, err := model.GetUserById(userID, false)
	if err == nil && !cprMemberAuthorizer(identity.UserID, c.GetInt("role"), target.Role) {
		err = model.ErrCprMemberAccessForbidden
	}
	if err != nil {
		cprMemberResponse(c, dto.CprMemberAccess{}, err)
		return 0, model.AuthSessionIdentity{}, false
	}
	return userID, session, true
}

func cprMemberResponse(c *gin.Context, view dto.CprMemberAccess, err error) string {
	if err == nil {
		common.ApiSuccess(c, view)
		return ""
	}
	status, code, message := http.StatusServiceUnavailable, "member_access_unavailable", "Member access is unavailable."
	switch {
	case errors.Is(err, service.ErrCprMemberAccessInvalid):
		status, code, message = http.StatusBadRequest, "member_access_invalid", "Invalid parameters"
	case errors.Is(err, service.ErrCprMemberAccessGroups):
		status, code, message = http.StatusBadRequest, "member_access_groups_invalid", "Choose enabled account groups for this member."
	case errors.Is(err, model.ErrCprMemberAccessConflict):
		status, code, message = http.StatusConflict, "member_access_conflict", "Member access changed. Refresh and review the current state."
	case errors.Is(err, model.ErrCprMemberUserDisabled):
		status, code, message = http.StatusConflict, "member_access_user_disabled", "The member must be enabled before granting access."
	case errors.Is(err, model.ErrCprMemberAccessForbidden):
		status, code, message = http.StatusForbidden, "member_access_forbidden", "No permission to access users of same or higher level"
	case errors.Is(err, model.ErrUserSessionInactive):
		status, code, message = http.StatusUnauthorized, "member_access_session_required", "A dashboard login session is required"
	case errors.Is(err, gorm.ErrRecordNotFound):
		status, code, message = http.StatusNotFound, "member_access_user_missing", "The requested member was not found."
	case errors.Is(err, service.ErrCprMemberAccessPending):
		code, message = "member_access_pending", "The member access change is pending. Refresh its status before reconciling."
	}
	fields := gin.H{"code": code}
	if code == "member_access_pending" && view.OperationID != "" {
		fields["data"] = view
	}
	common.ApiErrorStatus(c, status, common.NewMessage(message), fields)
	return code
}

func GetCprMemberAccess(c *gin.Context) {
	userID, _, ok := cprMemberTarget(c)
	if !ok {
		return
	}
	view, err := service.GetCprMemberAccess(c.Request.Context(), userID)
	cprMemberResponse(c, view, err)
}

func UpdateCprMemberAccess(c *gin.Context) {
	userID, identity, ok := cprMemberTarget(c)
	if !ok {
		return
	}
	var command dto.CprMemberGroupsCommand
	if !decodeAccountEgressCommand(c, &command, 64<<10) {
		return
	}
	view, err := service.SaveCprMemberAccess(c.Request.Context(), identity, userID, command, cprMemberAuthorizer)
	finishCprMemberAction(c, userID, "account_egress.member_groups", view, err)
}

func ReconcileCprMemberAccess(c *gin.Context) {
	userID, identity, ok := cprMemberTarget(c)
	if !ok {
		return
	}
	var command dto.CprMemberReconcileCommand
	if !decodeAccountEgressCommand(c, &command, 4<<10) {
		return
	}
	view, err := service.ReconcileCprMemberAccess(c.Request.Context(), identity, userID, command, cprMemberAuthorizer)
	finishCprMemberAction(c, userID, "account_egress.member_reconcile", view, err)
}

func finishCprMemberAction(c *gin.Context, userID int, action string, view dto.CprMemberAccess, err error) {
	code := cprMemberResponse(c, view, err)
	result := "success"
	if code == "member_access_pending" {
		result = "unknown"
	} else if err != nil {
		result = "failure"
	}
	accountEgressAudit(c, action, "member", strconv.Itoa(userID), view.OperationID, result, code)
}
