package controller

import (
	"errors"
	"net/http"
	"strings"
	"unicode"
	"unicode/utf8"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/middleware"
	"github.com/QuantumNous/new-api/model"
	"github.com/QuantumNous/new-api/service"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
)

// 授权只接受真实登录会话；管理员身份及会话版本来自认证中间件
func cprOAuthIdentity(c *gin.Context) (service.AuthIdentity, bool) {
	identity, ok := middleware.GetSessionAuthIdentity(c)
	if !ok {
		common.ApiErrorStatus(c, http.StatusUnauthorized, common.NewMessage("A dashboard login session is required"), gin.H{"code": "account_authorization_session_required"})
		return service.AuthIdentity{}, false
	}
	return identity, true
}

func cprOAuthResponse(c *gin.Context, data any, err error) {
	if err == nil {
		common.ApiSuccess(c, data)
		return
	}
	status, code, message := http.StatusServiceUnavailable, "account_authorization_unavailable", "Account authorization service is unavailable"
	switch {
	case errors.Is(err, model.ErrCprOAuthMissing), errors.Is(err, model.ErrCprOAuthOwner):
		status, code, message = http.StatusNotFound, "account_authorization_missing", "Account authorization operation is unavailable"
	case errors.Is(err, model.ErrCprOAuthConflict), errors.Is(err, model.ErrCprOAuthExpired):
		status, code, message = http.StatusConflict, "account_authorization_changed", "Account authorization has changed. Start a new operation."
	case errors.Is(err, model.ErrCprOAuthForbidden):
		status, code, message = http.StatusForbidden, "account_authorization_forbidden", "Account authorization permission is required"
	case errors.Is(err, model.ErrUserSessionInactive):
		status, code, message = http.StatusUnauthorized, "account_authorization_session_required", "A dashboard login session is required"
	}
	common.ApiErrorStatus(c, status, common.NewMessage(message), gin.H{"code": code})
}

func cprOAuthInvalid(c *gin.Context) {
	common.ApiErrorStatus(c, http.StatusBadRequest, common.NewMessage("Account authorization request is invalid"), gin.H{"code": "account_authorization_invalid"})
}

func cprOAuthOperationID(c *gin.Context) (string, bool) {
	id, err := uuid.Parse(c.Param("operation_id"))
	if err != nil {
		cprOAuthInvalid(c)
		return "", false
	}
	return id.String(), true
}

func StartCprOAuthOperation(c *gin.Context) {
	identity, ok := cprOAuthIdentity(c)
	if !ok {
		return
	}
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 16<<10)
	var input service.CprOAuthStartInput
	if err := common.DecodeJsonBody(c.Request.Body, &input); err != nil {
		cprOAuthInvalid(c)
		return
	}
	input.Name = strings.TrimSpace(input.Name)
	if input.Provider != "openai" && input.Provider != "xai" || input.Name == "" || utf8.RuneCountInString(input.Name) > 100 || strings.ContainsFunc(input.Name, unicode.IsControl) {
		cprOAuthInvalid(c)
		return
	}
	if input.AccountID != "" {
		if len(input.AccountID) > 256 || !strings.HasPrefix(input.AccountID, "acct_") || strings.ContainsFunc(input.AccountID, func(r rune) bool { return unicode.IsSpace(r) || unicode.IsControl(r) }) || input.SourceID != "" || input.NodeID != "" {
			cprOAuthInvalid(c)
			return
		}
	} else {
		source, e1 := uuid.Parse(input.SourceID)
		node, e2 := uuid.Parse(input.NodeID)
		if e1 != nil || e2 != nil {
			cprOAuthInvalid(c)
			return
		}
		input.SourceID = source.String()
		input.NodeID = node.String()
	}
	view, err := service.StartCprOAuthOperation(c.Request.Context(), identity, input)
	cprOAuthResponse(c, view, err)
	accountEgressOAuthAudit(c, "start", "", view, err)
}

func ImportCprAccountToken(c *gin.Context) {
	identity, ok := cprOAuthIdentity(c)
	if !ok {
		return
	}
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 32<<10)
	var input service.CprTokenImportInput
	if err := common.DecodeJsonBody(c.Request.Body, &input); err != nil {
		cprOAuthInvalid(c)
		return
	}
	input.Name = strings.TrimSpace(input.Name)
	input.AccessToken = strings.TrimSpace(input.AccessToken)
	if input.Provider != "openai" || input.Name == "" || utf8.RuneCountInString(input.Name) > 100 || strings.ContainsFunc(input.Name, unicode.IsControl) || input.AccessToken == "" || len(input.AccessToken) > 16<<10 || strings.ContainsFunc(input.AccessToken, func(r rune) bool { return unicode.IsSpace(r) || unicode.IsControl(r) }) {
		cprOAuthInvalid(c)
		return
	}
	source, e1 := uuid.Parse(input.SourceID)
	node, e2 := uuid.Parse(input.NodeID)
	if e1 != nil || e2 != nil {
		cprOAuthInvalid(c)
		return
	}
	input.SourceID, input.NodeID = source.String(), node.String()
	view, err := service.ImportCprAccountToken(c.Request.Context(), identity, input)
	input.AccessToken = ""
	cprOAuthResponse(c, view, err)
	accountEgressOAuthAudit(c, "import_token", "", view, err)
}

func ListCprOAuthOperations(c *gin.Context) {
	identity, ok := cprOAuthIdentity(c)
	if !ok {
		return
	}
	views, err := service.ListCprOAuthOperations(c.Request.Context(), identity)
	cprOAuthResponse(c, gin.H{"items": views}, err)
}

func GetCprOAuthOperation(c *gin.Context) {
	identity, ok := cprOAuthIdentity(c)
	if !ok {
		return
	}
	id, ok := cprOAuthOperationID(c)
	if !ok {
		return
	}
	view, err := service.ReadCprOAuthOperation(c.Request.Context(), identity, id)
	cprOAuthResponse(c, view, err)
}

func CompleteCprOAuthOperation(c *gin.Context) {
	identity, ok := cprOAuthIdentity(c)
	if !ok {
		return
	}
	id, ok := cprOAuthOperationID(c)
	if !ok {
		return
	}
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 32<<10)
	var input struct {
		CallbackURL string `json:"callback_url"`
	}
	if err := common.DecodeJsonBody(c.Request.Body, &input); err != nil {
		cprOAuthInvalid(c)
		return
	}
	input.CallbackURL = strings.TrimSpace(input.CallbackURL)
	if input.CallbackURL == "" || len(input.CallbackURL) > 16<<10 || strings.ContainsFunc(input.CallbackURL, unicode.IsControl) {
		cprOAuthInvalid(c)
		return
	}
	// 回调内容仅转交固定CPR服务端点，由Provider解析state与PKCE
	view, err := service.FinishCprOAuthOperation(c.Request.Context(), identity, id, "complete", input.CallbackURL)
	cprOAuthResponse(c, view, err)
	accountEgressOAuthAudit(c, "complete", id, view, err)
}

func CancelCprOAuthOperation(c *gin.Context) {
	identity, ok := cprOAuthIdentity(c)
	if !ok {
		return
	}
	id, ok := cprOAuthOperationID(c)
	if !ok {
		return
	}
	view, err := service.FinishCprOAuthOperation(c.Request.Context(), identity, id, "cancel", "")
	cprOAuthResponse(c, view, err)
	accountEgressOAuthAudit(c, "cancel", id, view, err)
}

func accountEgressOAuthAudit(c *gin.Context, action, id string, view service.CprOAuthView, err error) {
	if id == "" && canonicalAccountEgressUUID(&view.OperationID) {
		id = view.OperationID
	}
	result := "failure"
	if err == nil {
		switch {
		case action == "start" && view.Status == "pending",
			action == "import_token" && view.Status == "completed",
			action == "complete" && view.Status == "completed",
			action == "cancel" && view.Status == "cancelled":
			result = "success"
		case view.Status == "unknown", view.Status == "completing", view.Status == "pending":
			result = "unknown"
		}
	}
	accountEgressAudit(c, "account_egress.oauth_"+action, "authorization", id, id, result, "")
}
