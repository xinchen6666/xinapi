package service

import (
	"bytes"
	"context"
	"errors"
	"io"
	"net/http"
	"net/url"
	"time"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/model"
	"github.com/QuantumNous/new-api/service/authz"
	"github.com/google/uuid"
)

// CprOAuthStartInput仅接收管理员选择，身份和操作ID由服务端生成
type CprOAuthStartInput struct {
	Provider  string `json:"provider"`
	Name      string `json:"name"`
	AccountID string `json:"account_id,omitempty"`
	SourceID  string `json:"source_id,omitempty"`
	NodeID    string `json:"node_id,omitempty"`
}

// CprOAuthView是浏览器可见的脱敏操作与确认回执
type CprOAuthView struct {
	OperationID      string `json:"operation_id"`
	Status           string `json:"status"`
	Provider         string `json:"provider"`
	AccountID        string `json:"account_id,omitempty"`
	ExpiresAt        string `json:"expires_at"`
	SourceID         string `json:"source_id,omitempty"`
	NodeID           string `json:"node_id,omitempty"`
	VersionID        string `json:"version_id,omitempty"`
	ProxyID          string `json:"proxy_id,omitempty"`
	RouteGeneration  int64  `json:"route_generation"`
	AuthorizationURL string `json:"authorization_url,omitempty"`
	Reason           string `json:"reason,omitempty"`
}

var cprOAuthClient = &http.Client{
	Timeout:       90 * time.Second,
	Transport:     &http.Transport{Proxy: nil, MaxIdleConnsPerHost: 2, IdleConnTimeout: 30 * time.Second, ResponseHeaderTimeout: 85 * time.Second},
	CheckRedirect: func(_ *http.Request, _ []*http.Request) error { return http.ErrUseLastResponse },
}

func cprOAuthPermission(userID, role int) bool {
	return authz.Can(userID, role, authz.AccountEgressAuthorize)
}

func cprOAuthRequest(ctx context.Context, method, path string, input any) (CprOAuthView, error) {
	var view CprOAuthView
	if method == http.MethodGet {
		var cancel context.CancelFunc
		ctx, cancel = context.WithTimeout(ctx, 5*time.Second)
		defer cancel()
	}
	key, err := readCPRAdminKey()
	if err != nil {
		return view, ErrAccountEgressUnavailable
	}
	var payload []byte
	if input != nil {
		payload, err = common.Marshal(input)
		if err != nil {
			return view, ErrAccountEgressUnavailable
		}
	}
	request, err := http.NewRequestWithContext(ctx, method, "http://cpr:8080/api/admin/oauth-operations"+path, bytes.NewReader(payload))
	if err != nil {
		return view, ErrAccountEgressUnavailable
	}
	request.Header.Set("x-api-key", key)
	request.Header.Set("Accept", "application/json")
	if input != nil {
		request.Header.Set("Content-Type", "application/json")
	}
	response, err := cprOAuthClient.Do(request)
	if err != nil {
		return view, ErrAccountEgressUnavailable
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK && response.StatusCode != http.StatusCreated {
		return view, ErrAccountEgressUnavailable
	}
	var envelope struct {
		Code int           `json:"code"`
		Data *CprOAuthView `json:"data"`
	}
	body := &io.LimitedReader{R: response.Body, N: (64 << 10) + 1}
	if err := common.DecodeJsonBody(body, &envelope); err != nil || body.N == 0 || envelope.Data == nil || (envelope.Code != http.StatusOK && envelope.Code != http.StatusCreated) {
		return view, ErrAccountEgressUnavailable
	}
	view = *envelope.Data
	switch view.Status {
	case "pending", "completing", "completed", "failed", "unknown", "cancelled":
	default:
		return CprOAuthView{}, ErrAccountEgressUnavailable
	}
	if view.RouteGeneration < 0 {
		return CprOAuthView{}, ErrAccountEgressUnavailable
	}
	return view, nil
}

func cprOAuthView(operation *model.CprOAuthOperation) CprOAuthView {
	return CprOAuthView{OperationID: operation.OperationID, Status: operation.Status, Provider: operation.Provider, AccountID: operation.AccountID, ExpiresAt: time.Unix(operation.ExpiresAt, 0).UTC().Format(time.RFC3339), SourceID: operation.SourceID, NodeID: operation.NodeID, VersionID: operation.VersionID, ProxyID: operation.ProxyID, RouteGeneration: operation.RouteGeneration, Reason: operation.Reason}
}

func cprOAuthFailure(operation *model.CprOAuthOperation, reason string) CprOAuthView {
	operation.Status = "unknown"
	operation.Reason = reason
	return cprOAuthView(operation)
}

// cprOAuthApplyReceipt返回是否确认远端仍为pending，供显式取消释放启动结果未知的预留
// 已进入unknown的操作始终只允许确认终态或取消，完成交换需明确创建新操作
func cprOAuthApplyReceipt(operation *model.CprOAuthOperation, receipt CprOAuthView, starting bool) (CprOAuthView, bool) {
	now := time.Now().Unix()
	if receipt.OperationID != operation.OperationID || receipt.Provider != operation.Provider {
		return cprOAuthFailure(operation, "receipt_mismatch"), false
	}
	expiry, err := time.Parse(time.RFC3339, receipt.ExpiresAt)
	terminal := receipt.Status == "completed" || receipt.Status == "cancelled" || receipt.Status == "failed"
	if receipt.Status == "unknown" || err != nil || (!terminal && (expiry.Unix() <= now || operation.ExpiresAt <= now)) {
		return cprOAuthFailure(operation, "receipt_unknown"), false
	}
	if operation.TargetAccountID != "" && receipt.AccountID != operation.TargetAccountID {
		return cprOAuthFailure(operation, "receipt_mismatch"), false
	}
	firstBinding := operation.VersionID == "" && operation.AttemptAt == 0
	if firstBinding && operation.CancelAttemptAt > 0 && receipt.Status == "cancelled" {
		if operation.SourceID != "" && receipt.SourceID != operation.SourceID || operation.NodeID != "" && receipt.NodeID != operation.NodeID {
			return cprOAuthFailure(operation, "receipt_mismatch"), false
		}
		operation.AccountID = receipt.AccountID
		operation.SourceID = receipt.SourceID
		operation.NodeID = receipt.NodeID
		operation.VersionID = receipt.VersionID
		operation.ProxyID = receipt.ProxyID
		operation.RouteGeneration = receipt.RouteGeneration
		operation.ExpiresAt = expiry.Unix()
		operation.Status = "cancelled"
		operation.Reason = ""
		return cprOAuthView(operation), false
	}
	if starting || firstBinding {
		if receipt.Status != "pending" || receipt.AccountID == "" || receipt.SourceID == "" || receipt.NodeID == "" || receipt.VersionID == "" || receipt.ProxyID == "" || receipt.RouteGeneration <= 0 {
			return cprOAuthFailure(operation, "receipt_unknown"), false
		}
		if operation.SourceID != "" && receipt.SourceID != operation.SourceID || operation.NodeID != "" && receipt.NodeID != operation.NodeID {
			return cprOAuthFailure(operation, "receipt_mismatch"), false
		}
		operation.AccountID = receipt.AccountID
		operation.SourceID = receipt.SourceID
		operation.NodeID = receipt.NodeID
		operation.VersionID = receipt.VersionID
		operation.ProxyID = receipt.ProxyID
		operation.RouteGeneration = receipt.RouteGeneration
		operation.ExpiresAt = expiry.Unix()
		if !starting {
			return cprOAuthFailure(operation, "start_unknown"), true
		}
		authorization, err := url.Parse(receipt.AuthorizationURL)
		if err != nil || authorization.Host == "" || (authorization.Scheme != "http" && authorization.Scheme != "https") || authorization.User != nil {
			return cprOAuthFailure(operation, "start_unknown"), true
		}
		operation.Status = "pending"
		operation.Reason = ""
		view := cprOAuthView(operation)
		view.AuthorizationURL = receipt.AuthorizationURL
		return view, true
	}
	if receipt.AccountID != operation.AccountID || receipt.SourceID != operation.SourceID || receipt.NodeID != operation.NodeID || receipt.VersionID != operation.VersionID || receipt.ProxyID != operation.ProxyID || receipt.RouteGeneration != operation.RouteGeneration || expiry.Unix() != operation.ExpiresAt {
		return cprOAuthFailure(operation, "receipt_mismatch"), false
	}
	switch receipt.Status {
	case "completed", "failed", "cancelled":
		operation.Status = receipt.Status
		operation.Reason = ""
	case "pending":
		if operation.AttemptAt != 0 || operation.CancelAttemptAt != 0 || operation.Status != "pending" {
			return cprOAuthFailure(operation, "completion_unknown"), true
		}
		operation.Reason = ""
		return cprOAuthView(operation), true
	default:
		return cprOAuthFailure(operation, "completion_unknown"), false
	}
	return cprOAuthView(operation), false
}

func StartCprOAuthOperation(ctx context.Context, identity AuthIdentity, input CprOAuthStartInput) (CprOAuthView, error) {
	now := time.Now().Unix()
	operation := &model.CprOAuthOperation{OperationID: uuid.NewString(), UserID: identity.UserID, SessionID: identity.SessionID, UserAuthVersion: identity.UserAuthVersion, SessionVersion: identity.SessionVersion, Provider: input.Provider, Name: input.Name, TargetAccountID: input.AccountID, AccountID: input.AccountID, SourceID: input.SourceID, NodeID: input.NodeID, ExpiresAt: now + 15*60, Status: "unknown", CreatedAt: now, UpdatedAt: now, Revision: 1}
	if err := model.CreateCprOAuthOperation(identity, operation, cprOAuthPermission); err != nil {
		return CprOAuthView{}, err
	}
	// 客户端中止后仍以有界请求完成本次启动，所有后续恢复只查询既有回执
	remoteCtx, cancel := context.WithTimeout(context.WithoutCancel(ctx), 90*time.Second)
	defer cancel()
	payload := struct {
		OperationID string `json:"operation_id"`
		CprOAuthStartInput
	}{OperationID: operation.OperationID, CprOAuthStartInput: input}
	receipt, err := cprOAuthRequest(remoteCtx, http.MethodPost, "/start", payload)
	if err != nil {
		view, _, readErr := readCprOAuthReceipt(remoteCtx, operation)
		return view, readErr
	}
	view, _ := cprOAuthApplyReceipt(operation, receipt, true)
	saved, err := model.SaveCprOAuthReceipt(operation)
	if err != nil {
		return CprOAuthView{}, err
	}
	if !saved {
		return CprOAuthView{}, model.ErrCprOAuthConflict
	}
	return view, nil
}

// 回执查询仅更新本次调用的内存投影，数据库状态由启动和发送领取路径推进
func readCprOAuthReceipt(ctx context.Context, operation *model.CprOAuthOperation) (CprOAuthView, bool, error) {
	receipt, err := cprOAuthRequest(ctx, http.MethodGet, "?operation_id="+url.QueryEscape(operation.OperationID), nil)
	if err != nil {
		return cprOAuthFailure(operation, "receipt_unavailable"), false, nil
	}
	view, remotePending := cprOAuthApplyReceipt(operation, receipt, false)
	return view, remotePending, nil
}

func ReadCprOAuthOperation(ctx context.Context, identity AuthIdentity, id string) (CprOAuthView, error) {
	operation, err := model.GetCprOAuthOperation(identity, id, cprOAuthPermission)
	if err != nil {
		return CprOAuthView{}, err
	}
	view, _, err := readCprOAuthReceipt(ctx, operation)
	return view, err
}

func ListCprOAuthOperations(ctx context.Context, identity AuthIdentity) ([]CprOAuthView, error) {
	ctx, cancel := context.WithTimeout(ctx, 12*time.Second)
	defer cancel()
	operations, err := model.ListCprOAuthOperations(identity, cprOAuthPermission)
	if err != nil {
		return nil, err
	}
	views := make([]CprOAuthView, 0, len(operations))
	for i := range operations {
		view, _, err := readCprOAuthReceipt(ctx, &operations[i])
		if err != nil {
			return nil, err
		}
		views = append(views, view)
	}
	return views, nil
}

// Complete保留单次交换领取；Cancel只在管理员明确提交时执行幂等资源收尾
func FinishCprOAuthOperation(ctx context.Context, identity AuthIdentity, id, kind, callbackURL string) (CprOAuthView, error) {
	operation, err := model.GetCprOAuthOperation(identity, id, cprOAuthPermission)
	if err != nil {
		return CprOAuthView{}, err
	}
	storedStatus := operation.Status
	view, remotePending, err := readCprOAuthReceipt(ctx, operation)
	if err != nil {
		return CprOAuthView{}, err
	}
	if kind == "cancel" {
		if view.Status == "completed" || storedStatus == "completed" {
			return view, nil
		}
	} else {
		if kind != "complete" || storedStatus != "pending" || view.Status != "pending" || !remotePending || operation.AttemptAt != 0 || operation.CancelAttemptAt != 0 {
			return view, nil
		}
		if operation.ExpiresAt <= time.Now().Unix() {
			return CprOAuthView{}, model.ErrCprOAuthExpired
		}
	}
	claimed, err := model.ClaimCprOAuthOperation(identity, id, operation.Revision, kind, cprOAuthPermission)
	if err != nil {
		if errors.Is(err, model.ErrCprOAuthConflict) {
			return ReadCprOAuthOperation(ctx, identity, id)
		}
		return CprOAuthView{}, err
	}
	// 取消可使用领取前只读确认的预留事实，数据库领取仍保留原revision和单次发送标记
	operation.Status = claimed.Status
	operation.AttemptAt = claimed.AttemptAt
	operation.CancelAttemptAt = claimed.CancelAttemptAt
	operation.AttemptKind = claimed.AttemptKind
	operation.Revision = claimed.Revision
	operation.UpdatedAt = claimed.UpdatedAt
	claimed = operation
	remoteCtx, cancel := context.WithTimeout(context.WithoutCancel(ctx), 90*time.Second)
	defer cancel()
	payload := map[string]string{"operation_id": id}
	if kind == "complete" {
		payload["callback_url"] = callbackURL
	}
	receipt, sendErr := cprOAuthRequest(remoteCtx, http.MethodPost, "/"+kind, payload)
	if sendErr != nil {
		// 发送结果未知时仅GET确认，同一授权码永远保持一次POST
		view, _, err := readCprOAuthReceipt(remoteCtx, claimed)
		return view, err
	}
	view, _ = cprOAuthApplyReceipt(claimed, receipt, false)
	saved, err := model.SaveCprOAuthReceipt(claimed)
	if err != nil {
		return CprOAuthView{}, err
	}
	if !saved {
		return ReadCprOAuthOperation(ctx, identity, id)
	}
	return view, nil
}
