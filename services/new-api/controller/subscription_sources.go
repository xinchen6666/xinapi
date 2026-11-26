package controller

import (
	"errors"
	"net/http"
	"net/url"
	"strings"
	"unicode/utf8"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/dto"
	"github.com/QuantumNous/new-api/service"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
)

const maxSourceRevision uint64 = 9007199254740991

func sourceRevisionValid(id string, revision uint64) bool {
	_, err := uuid.Parse(id)
	return err == nil && revision > 0 && revision <= maxSourceRevision
}
func sourceNameValid(name string) bool { return name != "" && utf8.RuneCountInString(name) <= 100 }
func sourceURLValid(value string) bool {
	parsed, err := url.Parse(value)
	return err == nil && len(value) <= maxSubscriptionSourceURLBytes && parsed.Host != "" && (strings.EqualFold(parsed.Scheme, "http") || strings.EqualFold(parsed.Scheme, "https"))
}
func sourceRequestInvalid(c *gin.Context) {
	common.ApiErrorStatus(c, http.StatusBadRequest, common.NewMessage("Review the subscription source fields and approval decisions."), gin.H{"code": "subscription_source_request_invalid"})
}
func sourceResponse(c *gin.Context, data any, err error) {
	if err != nil {
		var sourceError *service.SubscriptionSourceError
		if errors.As(err, &sourceError) {
			common.ApiErrorStatus(c, sourceError.Status, sourceError.Message, gin.H{"code": sourceError.Code})
			return
		}
		common.ApiErrorStatus(c, http.StatusServiceUnavailable, common.NewMessage("Account and egress service is unavailable"), gin.H{"code": "account_egress_unavailable"})
		return
	}
	common.ApiSuccess(c, data)
}
func GetSubscriptionSources(c *gin.Context) {
	data, err := service.ReadSubscriptionSources(c.Request.Context())
	if data.Items == nil {
		data.Items = []dto.SubscriptionSourceSummary{}
	}
	sourceResponse(c, data, err)
}
func GetSubscriptionSourceDetail(c *gin.Context) {
	id := c.Query("source_id")
	if _, err := uuid.Parse(id); err != nil {
		sourceRequestInvalid(c)
		return
	}
	data, err := service.ReadSubscriptionSourceDetail(c.Request.Context(), id)
	sourceResponse(c, data, err)
}
func CreateSubscriptionSource(c *gin.Context) {
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 16<<10)
	var input dto.SubscriptionSourceCreate
	if err := c.ShouldBindJSON(&input); err != nil {
		sourceRequestInvalid(c)
		return
	}
	input.Name = strings.TrimSpace(input.Name)
	input.SourceURL = strings.TrimSpace(input.SourceURL)
	if !sourceNameValid(input.Name) || !sourceURLValid(input.SourceURL) {
		sourceRequestInvalid(c)
		return
	}
	data, err := service.CreateSubscriptionSource(c.Request.Context(), input)
	sourceResponse(c, data, err)
}
func UpdateSubscriptionSource(c *gin.Context) {
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 16<<10)
	var input dto.SubscriptionSourceUpdate
	if err := c.ShouldBindJSON(&input); err != nil {
		sourceRequestInvalid(c)
		return
	}
	input.Name = strings.TrimSpace(input.Name)
	if input.SourceURL != nil {
		value := strings.TrimSpace(*input.SourceURL)
		input.SourceURL = nil
		if value != "" {
			if !sourceURLValid(value) {
				sourceRequestInvalid(c)
				return
			}
			input.SourceURL = &value
		}
	}
	if !sourceRevisionValid(input.SourceID, input.ExpectedRevision) || !sourceNameValid(input.Name) {
		sourceRequestInvalid(c)
		return
	}
	data, err := service.UpdateSubscriptionSource(c.Request.Context(), input)
	sourceResponse(c, data, err)
}
func RefreshSubscriptionSource(c *gin.Context) {
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 4096)
	var input dto.SubscriptionSourceRefresh
	if err := c.ShouldBindJSON(&input); err != nil || !sourceRevisionValid(input.SourceID, input.ExpectedRevision) {
		sourceRequestInvalid(c)
		return
	}
	data, err := service.RefreshSubscriptionSource(c.Request.Context(), input)
	sourceResponse(c, data, err)
}
func ApproveSubscriptionSource(c *gin.Context) {
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 512<<10)
	var input dto.SubscriptionSourceApprove
	if err := c.ShouldBindJSON(&input); err != nil {
		sourceRequestInvalid(c)
		return
	}
	_, reviewErr := uuid.Parse(input.ReviewID)
	if !sourceRevisionValid(input.SourceID, input.ExpectedRevision) || reviewErr != nil || input.Selected == nil || len(input.Selected) > 4096 {
		sourceRequestInvalid(c)
		return
	}
	candidates := make(map[int]bool, len(input.Selected))
	nodes := make(map[string]bool, len(input.Selected))
	for _, selected := range input.Selected {
		if selected.CandidateIndex < 0 || selected.CandidateIndex >= 4096 || candidates[selected.CandidateIndex] {
			sourceRequestInvalid(c)
			return
		}
		candidates[selected.CandidateIndex] = true
		if selected.NodeID != nil {
			id, err := uuid.Parse(*selected.NodeID)
			if err != nil || nodes[id.String()] {
				sourceRequestInvalid(c)
				return
			}
			nodes[id.String()] = true
		}
	}
	data, err := service.ApproveSubscriptionSource(c.Request.Context(), input)
	sourceResponse(c, data, err)
}
