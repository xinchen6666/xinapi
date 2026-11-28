package service

import (
	"context"
	"net/http"
	"time"

	"github.com/QuantumNous/new-api/model"
	"github.com/google/uuid"
)

// Token 导入的明文仅用于一次固定 CPR 请求，持久记录沿用脱敏授权回执
type CprTokenImportInput struct {
	Provider    string `json:"provider"`
	Name        string `json:"name"`
	SourceID    string `json:"source_id"`
	NodeID      string `json:"node_id"`
	AccessToken string `json:"access_token"`
}

func ImportCprAccountToken(ctx context.Context, identity AuthIdentity, input CprTokenImportInput) (CprOAuthView, error) {
	now := time.Now().Unix()
	operation := &model.CprOAuthOperation{OperationID: uuid.NewString(), UserID: identity.UserID, SessionID: identity.SessionID, UserAuthVersion: identity.UserAuthVersion, SessionVersion: identity.SessionVersion, Provider: input.Provider, Name: input.Name, SourceID: input.SourceID, NodeID: input.NodeID, ExpiresAt: now + 15*60, Status: "unknown", AttemptKind: "token_import", AttemptAt: now, CreatedAt: now, UpdatedAt: now, Revision: 1}
	if err := model.CreateCprOAuthOperation(identity, operation, cprOAuthPermission); err != nil {
		return CprOAuthView{}, err
	}
	remoteCtx, cancel := context.WithTimeout(context.WithoutCancel(ctx), 90*time.Second)
	defer cancel()
	payload := struct {
		OperationID string `json:"operation_id"`
		CprTokenImportInput
	}{OperationID: operation.OperationID, CprTokenImportInput: input}
	receipt, err := cprOAuthRequest(remoteCtx, http.MethodPost, "/import-token", nil, payload)
	payload.AccessToken = ""
	input.AccessToken = ""
	if err != nil {
		view, _, readErr := readCprOAuthReceipt(remoteCtx, operation)
		return view, readErr
	}
	view, _ := cprOAuthApplyReceipt(operation, receipt, false)
	saved, err := model.SaveCprOAuthReceipt(operation)
	if err != nil {
		return CprOAuthView{}, err
	}
	if !saved {
		return CprOAuthView{}, model.ErrCprOAuthConflict
	}
	return view, nil
}
