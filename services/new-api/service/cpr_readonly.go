package service

import (
	"context"
	"errors"
	"io"
	"net/http"
	"net/url"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/dto"
)

var ErrAccountEgressUnavailable = errors.New("Account and egress service is unavailable")

// 内部管理请求使用固定地址与独立传输，凭据仅发送到 CPR。
var cprReadOnlyClient = &http.Client{
	Timeout: 8 * time.Second,
	Transport: &http.Transport{
		Proxy:                 nil,
		MaxIdleConnsPerHost:   2,
		IdleConnTimeout:       30 * time.Second,
		ResponseHeaderTimeout: 5 * time.Second,
	},
	CheckRedirect: func(_ *http.Request, _ []*http.Request) error {
		return http.ErrUseLastResponse
	},
}

func ReadAccountEgressAccounts(ctx context.Context, page, pageSize int) (dto.AccountEgressPage[dto.AccountEgressAccount], error) {
	result, err := readCPRList[dto.AccountEgressAccount](ctx, "/api/admin/accounts", page, pageSize)
	if err != nil {
		return result, err
	}
	// OAuth 导入可使用邮箱作为默认名称，管理列表统一展示稳定账号别名。
	for i := range result.Items {
		if strings.Contains(result.Items[i].Name, "@") {
			result.Items[i].Name = result.Items[i].ID
		}
	}
	return result, nil
}

func ReadAccountEgressProxies(ctx context.Context, page, pageSize int) (dto.AccountEgressPage[dto.AccountEgressProxy], error) {
	return readCPRList[dto.AccountEgressProxy](ctx, "/api/admin/proxies", page, pageSize)
}

// readCPRAdminKey 只接受本机受保护的规则文件，凭据留在进程内存。
func readCPRAdminKey() (string, error) {
	keyPath := os.Getenv("XINAPI_CPR_ADMIN_KEY_FILE")
	info, err := os.Lstat(keyPath)
	if err != nil || !info.Mode().IsRegular() || info.Mode().Perm()&0077 != 0 {
		return "", ErrAccountEgressUnavailable
	}
	keyFile, err := os.Open(keyPath)
	if err != nil {
		return "", ErrAccountEgressUnavailable
	}
	defer keyFile.Close()
	info, err = keyFile.Stat()
	if err != nil || !info.Mode().IsRegular() || info.Mode().Perm()&0077 != 0 {
		return "", ErrAccountEgressUnavailable
	}
	keyBytes, err := io.ReadAll(io.LimitReader(keyFile, 4097))
	if err != nil || len(keyBytes) > 4096 {
		return "", ErrAccountEgressUnavailable
	}
	key := strings.TrimSpace(string(keyBytes))
	if key == "" || strings.ContainsAny(key, "\r\n") {
		return "", ErrAccountEgressUnavailable
	}
	return key, nil
}

func readCPRList[T any](ctx context.Context, path string, page, pageSize int) (dto.AccountEgressPage[T], error) {
	var result dto.AccountEgressPage[T]
	query := url.Values{"page": {strconv.Itoa(page)}, "pageSize": {strconv.Itoa(pageSize)}}
	response, err := sendCPRAdmin(ctx, cprReadOnlyClient, http.MethodGet, path, query, nil)
	if err != nil {
		return result, ErrAccountEgressUnavailable
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return result, ErrAccountEgressUnavailable
	}
	// 列表按白名单结构解码，CPR 的原始错误、地址和凭据保持在内部。
	var envelope struct {
		Code int                       `json:"code"`
		Data *dto.AccountEgressPage[T] `json:"data"`
	}
	body := &io.LimitedReader{R: response.Body, N: (8 << 20) + 1}
	if err := common.DecodeJsonBody(body, &envelope); err != nil || body.N == 0 || envelope.Code != http.StatusOK || envelope.Data == nil {
		return result, ErrAccountEgressUnavailable
	}
	result = *envelope.Data
	if result.Items == nil {
		result.Items = []T{}
	}
	return result, nil
}

// 订阅预览要等 CPR 完成下载与解析，为它的 10 秒预算预留更长的总时限。
const (
	cprSubscriptionPreviewTimeout       = 20 * time.Second
	cprSubscriptionPreviewHeaderTimeout = 15 * time.Second
	cprSubscriptionPreviewBodyLimit     = 8 << 20
	cprSubscriptionPreviewErrorLimit    = 4 << 10
)

// 预览请求沿用固定内部地址与独立传输，凭据只发往 CPR。
var cprSubscriptionPreviewClient = &http.Client{
	Timeout: cprSubscriptionPreviewTimeout,
	Transport: &http.Transport{
		Proxy:                 nil,
		MaxIdleConnsPerHost:   2,
		IdleConnTimeout:       30 * time.Second,
		ResponseHeaderTimeout: cprSubscriptionPreviewHeaderTimeout,
	},
	CheckRedirect: func(_ *http.Request, _ []*http.Request) error {
		return http.ErrUseLastResponse
	},
}

// SubscriptionPreviewError 表示 CPR 拒绝的一次预览请求。Code 是稳定机器码，
// 供管理端区分失败原因；Message 是英文源串，由前端按语言翻译。
type SubscriptionPreviewError struct {
	Code    string
	Message *common.Message
}

func (e *SubscriptionPreviewError) Error() string {
	return e.Message.Error()
}

// Unwrap 让 common.ApiErrorStatus 识别内嵌的管理端消息。
func (e *SubscriptionPreviewError) Unwrap() error {
	return e.Message
}

// subscriptionPreviewReasonMessages 把 CPR 的稳定 reason 映射为管理端可读消息。
var subscriptionPreviewReasonMessages = map[string]string{
	"subscription_source_invalid":     "The subscription link must be an http or https address",
	"subscription_target_not_allowed": "The subscription link target is disabled for this deployment",
	"subscription_unreadable":         "The subscription document could not be read",
	"subscription_too_large":          "The subscription document is larger than the preview limit",
	"subscription_preview_too_large":  "The subscription preview exceeds the result limit",
	"subscription_timeout":            "The subscription download timed out",
	"subscription_too_many_redirects": "The subscription link redirects too many times",
	"subscription_fetch_failed":       "The subscription document could not be downloaded",
}

// ReadSubscriptionPreview 请 CPR 下载并解析一份订阅链接，返回预览白名单字段。
func ReadSubscriptionPreview(ctx context.Context, sourceURL string) (dto.AccountEgressSubscriptionPreview, error) {
	var preview dto.AccountEgressSubscriptionPreview
	command := struct {
		SourceURL string `json:"source_url"`
	}{SourceURL: sourceURL}
	response, err := sendCPRAdmin(ctx, cprSubscriptionPreviewClient, http.MethodPost, "/api/admin/subscriptions/preview", nil, command)
	if err != nil {
		return preview, ErrAccountEgressUnavailable
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return preview, readSubscriptionPreviewFailure(response)
	}
	// 预览按白名单结构解码，CPR 的其余字段与内部细节留在 CPR。
	var envelope struct {
		Code int                                   `json:"code"`
		Data *dto.AccountEgressSubscriptionPreview `json:"data"`
	}
	body := &io.LimitedReader{R: response.Body, N: cprSubscriptionPreviewBodyLimit + 1}
	if err := common.DecodeJsonBody(body, &envelope); err != nil || body.N == 0 || envelope.Code != http.StatusOK || envelope.Data == nil {
		return preview, ErrAccountEgressUnavailable
	}
	preview = *envelope.Data
	if preview.Candidates == nil {
		preview.Candidates = []dto.AccountEgressSubscriptionCandidate{}
	}
	if preview.Diagnostics == nil {
		preview.Diagnostics = []dto.AccountEgressSubscriptionDiagnostic{}
	}
	return preview, nil
}

// readSubscriptionPreviewFailure 只读取 CPR 的稳定 reason 与状态码，正文留在 CPR。
func readSubscriptionPreviewFailure(response *http.Response) error {
	var failure struct {
		Reason string `json:"reason"`
	}
	body := &io.LimitedReader{R: response.Body, N: cprSubscriptionPreviewErrorLimit + 1}
	if err := common.DecodeJsonBody(body, &failure); err == nil {
		if messageKey, ok := subscriptionPreviewReasonMessages[failure.Reason]; ok {
			return &SubscriptionPreviewError{Code: failure.Reason, Message: common.NewMessage(messageKey)}
		}
	}
	// 422 与 502 表示订阅内容被拒绝或下载失败，可向管理员说明原因。
	if response.StatusCode == http.StatusUnprocessableEntity || response.StatusCode == http.StatusBadGateway {
		return &SubscriptionPreviewError{
			Code:    "subscription_preview_failed",
			Message: common.NewMessage("The subscription preview could not be completed"),
		}
	}
	return ErrAccountEgressUnavailable
}
