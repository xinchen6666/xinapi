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
)

const maxSourceRevision uint64 = 9007199254740991

func sourceRevisionValid(id *string, revision uint64) bool {
	return canonicalAccountEgressUUID(id) && revision > 0 && revision <= maxSourceRevision
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
	var transportError *service.CPRManagementError
	if errors.As(err, &transportError) {
		accountEgressResponse(c, nil, err)
		return
	}
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
	if !canonicalAccountEgressUUID(&id) {
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
	finishSubscriptionSourceAction(c, data, err, "account_egress.source_create", "")
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
	if !sourceRevisionValid(&input.SourceID, input.ExpectedRevision) || !sourceNameValid(input.Name) {
		sourceRequestInvalid(c)
		return
	}
	data, err := service.UpdateSubscriptionSource(c.Request.Context(), input)
	finishSubscriptionSourceAction(c, data, err, "account_egress.source_update", input.SourceID)
}
func RefreshSubscriptionSource(c *gin.Context) {
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 4096)
	var input dto.SubscriptionSourceRefresh
	if err := c.ShouldBindJSON(&input); err != nil || !sourceRevisionValid(&input.SourceID, input.ExpectedRevision) {
		sourceRequestInvalid(c)
		return
	}
	data, err := service.RefreshSubscriptionSource(c.Request.Context(), input)
	finishSubscriptionSourceAction(c, data, err, "account_egress.source_refresh", input.SourceID)
}
func ApproveSubscriptionSource(c *gin.Context) {
	c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, 512<<10)
	var input dto.SubscriptionSourceApprove
	if err := c.ShouldBindJSON(&input); err != nil {
		sourceRequestInvalid(c)
		return
	}
	if !sourceRevisionValid(&input.SourceID, input.ExpectedRevision) || !canonicalAccountEgressUUID(&input.ReviewID) || input.Selected == nil || len(input.Selected) > 4096 {
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
			if !canonicalAccountEgressUUID(selected.NodeID) || nodes[*selected.NodeID] {
				sourceRequestInvalid(c)
				return
			}
			nodes[*selected.NodeID] = true
		}
	}
	data, err := service.ApproveSubscriptionSource(c.Request.Context(), input)
	finishSubscriptionSourceAction(c, data, err, "account_egress.source_approve", input.SourceID)
}

func finishSubscriptionSourceAction(c *gin.Context, data dto.SubscriptionSourceDetail, err error, action, sourceID string) {
	result, code := "success", ""
	if err != nil {
		result, code = accountEgressErrorOutcome(err)
	} else {
		if sourceID == "" && canonicalAccountEgressUUID(&data.Source.ID) {
			sourceID = data.Source.ID
		}
		if action == "account_egress.source_refresh" && data.Source.Status == "stale" {
			result, code = "failure", "subscription_refresh_failed"
		}
	}
	sourceResponse(c, data, err)
	accountEgressAudit(c, action, "subscription_source", sourceID, "", result, code)
}
