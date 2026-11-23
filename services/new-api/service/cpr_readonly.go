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

func readCPRList[T any](ctx context.Context, path string, page, pageSize int) (dto.AccountEgressPage[T], error) {
	var result dto.AccountEgressPage[T]
	keyPath := os.Getenv("XINAPI_CPR_ADMIN_KEY_FILE")
	info, err := os.Lstat(keyPath)
	if err != nil || !info.Mode().IsRegular() || info.Mode().Perm()&0077 != 0 {
		return result, ErrAccountEgressUnavailable
	}
	keyFile, err := os.Open(keyPath)
	if err != nil {
		return result, ErrAccountEgressUnavailable
	}
	defer keyFile.Close()
	info, err = keyFile.Stat()
	if err != nil || !info.Mode().IsRegular() || info.Mode().Perm()&0077 != 0 {
		return result, ErrAccountEgressUnavailable
	}
	keyBytes, err := io.ReadAll(io.LimitReader(keyFile, 4097))
	if err != nil || len(keyBytes) > 4096 {
		return result, ErrAccountEgressUnavailable
	}
	key := strings.TrimSpace(string(keyBytes))
	if key == "" || strings.ContainsAny(key, "\r\n") {
		return result, ErrAccountEgressUnavailable
	}
	query := url.Values{"page": {strconv.Itoa(page)}, "pageSize": {strconv.Itoa(pageSize)}}
	request, err := http.NewRequestWithContext(ctx, http.MethodGet, "http://cpr:8080"+path+"?"+query.Encode(), nil)
	if err != nil {
		return result, ErrAccountEgressUnavailable
	}
	request.Header.Set("x-api-key", key)
	request.Header.Set("Accept", "application/json")
	response, err := cprReadOnlyClient.Do(request)
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
