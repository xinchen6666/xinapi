package controller

import (
	"errors"
	"net/http"
	"net/url"
	"strconv"
	"strings"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/service"
	"github.com/gin-gonic/gin"
)

// 订阅预览只服务管理员的一次粘贴，按链接长度与协议限定请求。
const maxSubscriptionSourceURLBytes = 2048

type subscriptionPreviewRequest struct {
	SourceURL string `json:"source_url"`
}

func accountEgressPagination(c *gin.Context) (int, int, bool) {
	page, pageError := strconv.ParseUint(c.DefaultQuery("page", "1"), 10, 32)
	pageSize, sizeError := strconv.ParseUint(c.DefaultQuery("pageSize", "20"), 10, 16)
	if pageError != nil || sizeError != nil || page < 1 || pageSize < 1 || pageSize > 200 {
		common.ApiErrorStatus(c, http.StatusBadRequest, common.NewMessage("Invalid pagination parameters"))
		return 0, 0, false
	}
	return int(page), int(pageSize), true
}

func GetAccountEgressAccounts(c *gin.Context) {
	page, pageSize, valid := accountEgressPagination(c)
	if !valid {
		return
	}
	data, err := service.ReadAccountEgressAccounts(c.Request.Context(), page, pageSize)
	if err != nil {
		common.ApiErrorStatus(c, http.StatusServiceUnavailable, common.NewMessage("Account and egress service is unavailable"))
		return
	}
	common.ApiSuccess(c, data)
}

func GetAccountEgressProxies(c *gin.Context) {
	page, pageSize, valid := accountEgressPagination(c)
	if !valid {
		return
	}
	data, err := service.ReadAccountEgressProxies(c.Request.Context(), page, pageSize)
	if err != nil {
		common.ApiErrorStatus(c, http.StatusServiceUnavailable, common.NewMessage("Account and egress service is unavailable"))
		return
	}
	common.ApiSuccess(c, data)
}

func PreviewAccountEgressSubscription(c *gin.Context) {
	var request subscriptionPreviewRequest
	if err := c.ShouldBindJSON(&request); err != nil {
		common.ApiErrorStatus(c, http.StatusBadRequest, common.NewMessage("Invalid parameters"), gin.H{"code": "subscription_request_invalid"})
		return
	}
	sourceURL := strings.TrimSpace(request.SourceURL)
	if sourceURL == "" {
		common.ApiErrorStatus(c, http.StatusBadRequest, common.NewMessage("Enter a subscription link."), gin.H{"code": "subscription_source_required"})
		return
	}
	if len(sourceURL) > maxSubscriptionSourceURLBytes {
		common.ApiErrorStatus(c, http.StatusBadRequest,
			common.NewMessage("The subscription link must be at most {{max}} characters", map[string]any{"max": maxSubscriptionSourceURLBytes}),
			gin.H{"code": "subscription_source_too_long"})
		return
	}
	parsed, parseErr := url.Parse(sourceURL)
	if parseErr != nil || parsed.Host == "" || (!strings.EqualFold(parsed.Scheme, "http") && !strings.EqualFold(parsed.Scheme, "https")) {
		common.ApiErrorStatus(c, http.StatusBadRequest, common.NewMessage("The subscription link must be an http or https address"), gin.H{"code": "subscription_source_invalid"})
		return
	}
	data, err := service.ReadSubscriptionPreview(c.Request.Context(), sourceURL)
	if err != nil {
		var previewErr *service.SubscriptionPreviewError
		if errors.As(err, &previewErr) {
			common.ApiErrorStatus(c, http.StatusUnprocessableEntity, previewErr.Message, gin.H{"code": previewErr.Code})
			return
		}
		common.ApiErrorStatus(c, http.StatusServiceUnavailable, common.NewMessage("Account and egress service is unavailable"), gin.H{"code": "account_egress_unavailable"})
		return
	}
	common.ApiSuccess(c, data)
}
