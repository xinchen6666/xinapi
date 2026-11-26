package service

import (
	"bytes"
	"context"
	"errors"
	"io"
	"net/http"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/dto"
)

// SubscriptionSourceError 仅向管理页面返回固定消息与稳定错误码。
type SubscriptionSourceError struct {
	Status  int
	Code    string
	Message *common.Message
}

func (e *SubscriptionSourceError) Error() string { return e.Message.Error() }
func (e *SubscriptionSourceError) Unwrap() error { return e.Message }

func ReadSubscriptionSources(ctx context.Context) (dto.SubscriptionSourceList, error) {
	return requestSubscriptionSource[dto.SubscriptionSourceList](ctx, http.MethodGet, "", nil)
}
func ReadSubscriptionSourceDetail(ctx context.Context, sourceID string) (dto.SubscriptionSourceDetail, error) {
	return requestSubscriptionSource[dto.SubscriptionSourceDetail](ctx, http.MethodGet, "/detail?source_id="+sourceID, nil)
}
func CreateSubscriptionSource(ctx context.Context, input dto.SubscriptionSourceCreate) (dto.SubscriptionSourceDetail, error) {
	return requestSubscriptionSource[dto.SubscriptionSourceDetail](ctx, http.MethodPost, "/create", input)
}
func UpdateSubscriptionSource(ctx context.Context, input dto.SubscriptionSourceUpdate) (dto.SubscriptionSourceDetail, error) {
	return requestSubscriptionSource[dto.SubscriptionSourceDetail](ctx, http.MethodPost, "/update", input)
}
func RefreshSubscriptionSource(ctx context.Context, input dto.SubscriptionSourceRefresh) (dto.SubscriptionSourceDetail, error) {
	return requestSubscriptionSource[dto.SubscriptionSourceDetail](ctx, http.MethodPost, "/refresh", input)
}
func ApproveSubscriptionSource(ctx context.Context, input dto.SubscriptionSourceApprove) (dto.SubscriptionSourceDetail, error) {
	return requestSubscriptionSource[dto.SubscriptionSourceDetail](ctx, http.MethodPost, "/approve", input)
}

func requestSubscriptionSource[T any](ctx context.Context, method, path string, input any) (T, error) {
	var result T
	key, err := readCPRAdminKey()
	if err != nil {
		return result, err
	}
	var payload []byte
	if input != nil {
		payload, err = common.Marshal(input)
		if err != nil {
			return result, ErrAccountEgressUnavailable
		}
	}
	request, err := http.NewRequestWithContext(ctx, method, "http://cpr:8080/api/admin/subscriptions/sources"+path, bytes.NewReader(payload))
	if err != nil {
		return result, ErrAccountEgressUnavailable
	}
	request.Header.Set("x-api-key", key)
	request.Header.Set("Accept", "application/json")
	if input != nil {
		request.Header.Set("Content-Type", "application/json")
	}
	response, err := cprSubscriptionPreviewClient.Do(request)
	if err != nil {
		return result, ErrAccountEgressUnavailable
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		status := response.StatusCode
		if status == http.StatusConflict {
			return result, &SubscriptionSourceError{Status: status, Code: "subscription_source_conflict", Message: common.NewMessage("Subscription source changed. Refresh its details before approving.")}
		}
		if status == http.StatusNotFound {
			return result, &SubscriptionSourceError{Status: status, Code: "subscription_source_missing", Message: common.NewMessage("Subscription source is unavailable. Refresh the source list.")}
		}
		if status == http.StatusBadRequest || status == http.StatusUnprocessableEntity {
			failure := readSubscriptionPreviewFailure(response)
			var previewError *SubscriptionPreviewError
			if errors.As(failure, &previewError) {
				return result, &SubscriptionSourceError{Status: status, Code: previewError.Code, Message: previewError.Message}
			}
			return result, &SubscriptionSourceError{Status: status, Code: "subscription_source_request_invalid", Message: common.NewMessage("Review the subscription source fields and approval decisions.")}
		}
		return result, ErrAccountEgressUnavailable
	}
	var envelope struct {
		Code int `json:"code"`
		Data *T  `json:"data"`
	}
	body := &io.LimitedReader{R: response.Body, N: cprSubscriptionPreviewBodyLimit + 1}
	if err := common.DecodeJsonBody(body, &envelope); err != nil || body.N == 0 || envelope.Code != http.StatusOK || envelope.Data == nil {
		return result, ErrAccountEgressUnavailable
	}
	return *envelope.Data, nil
}
