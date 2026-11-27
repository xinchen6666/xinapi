package service

import (
	"fmt"
	"strings"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/model"
	relaycommon "github.com/QuantumNous/new-api/relay/common"
	"github.com/QuantumNous/new-api/relaykit/dto"
	"github.com/QuantumNous/new-api/relaykit/relayconvert"
	"github.com/QuantumNous/new-api/relaykit/types"
	"github.com/QuantumNous/new-api/tokenkit"
)

// ResponsesUsageAccumulator 汇总单次 Responses 数量，传输拥有者继续负责工具计数与一次结算
// SSE 与 WS 按原事件观察，Finish 同时保存数量的确认或估算状态
type ResponsesUsageAccumulator struct {
	info              *relaycommon.RelayInfo
	usage             *dto.Usage
	outputText        strings.Builder
	imageCounter      relaycommon.ImageGenerationCallCounter
	imageCommitted    bool
	started           bool
	finished          bool
	failed            bool
	inputReported     bool
	outputReported    bool
	upstreamEstimated bool
}

func NewResponsesUsageAccumulator(info *relaycommon.RelayInfo) *ResponsesUsageAccumulator {
	info.ResponsesUsageStatus = relaycommon.ResponsesUsageUnknown
	return &ResponsesUsageAccumulator{info: info, usage: &dto.Usage{}}
}

// CompleteResponsesUsage 让同步 JSON 与流式响应采用同一数量收尾，工具计数保留原调用方归属
func CompleteResponsesUsage(info *relaycommon.RelayInfo, response *dto.OpenAIResponsesResponse, raw []byte) *dto.Usage {
	accumulator := NewResponsesUsageAccumulator(info)
	accumulator.imageCommitted = true
	if response != nil {
		accumulator.started = response.ID != "" || len(response.Status) > 0 || response.Usage != nil || len(response.Output) > 0
		var status string
		_ = common.Unmarshal(response.Status, &status)
		accumulator.failed = status == "failed"
		accumulator.observeUsage(response.Usage, raw)
		accumulator.outputText.WriteString(responsesGeneratedText(response))
		ObserveResponsesOutcome(info, &dto.ResponsesStreamResponse{Response: response})
	}
	return accumulator.Finish()
}

// Observe 读取同一事件的原文，保留上游显式零值并沿用现有工具事实
func (a *ResponsesUsageAccumulator) Observe(event *dto.ResponsesStreamResponse, raw []byte) {
	if a == nil || event == nil || a.finished {
		return
	}
	if event.Response != nil {
		a.info.ObserveResponseModel(event.Response.Model)
	}
	a.started = true
	if event.Type == "error" || event.Type == "response.failed" || event.Type == "response.error" {
		a.failed = true
	}
	if event.Response != nil {
		var status string
		_ = common.Unmarshal(event.Response.Status, &status)
		a.failed = a.failed || status == "failed"
	}
	ObserveResponsesOutcome(a.info, event)
	switch event.Type {
	case "response.completed", "response.done", "response.failed", "response.incomplete", "response.cancelled", "response.canceled":
		if event.Response != nil {
			a.observeUsage(event.Response.Usage, raw)
			if a.outputText.Len() == 0 {
				a.outputText.WriteString(responsesGeneratedText(event.Response))
			}
		}
		a.info.ApplyVendorToolUsage(raw)
		if a.imageCommitted {
			return
		}
		if event.Response != nil {
			for i := range event.Response.Output {
				a.imageCounter.Observe(&event.Response.Output[i], &i)
			}
		}
		a.imageCounter.Commit(a.info)
		a.imageCommitted = true
	case "response.output_text.delta", "response.function_call_arguments.delta",
		"response.reasoning_summary_text.delta", "response.reasoning_text.delta", "response.refusal.delta":
		a.outputText.WriteString(event.Delta)
	case dto.ResponsesOutputTypeItemDone:
		if event.Item == nil {
			return
		}
		switch event.Item.Type {
		case dto.BuildInCallWebSearchCall, dto.BuildInCallFileSearchCall, dto.BuildInCallFunctionCall:
			a.info.CountBillableToolCall(event.Item.Type, event.Item.Name)
		case dto.ResponsesOutputTypeImageGenerationCall:
			if !a.imageCommitted {
				a.imageCounter.Observe(event.Item, event.OutputIndex)
			}
		}
	}
}

// responsesUsageCounts 仅保留计数字段的存在性，显式零值与缺字段分别处理
type responsesUsageCounts struct {
	Input      *int `json:"input_tokens"`
	Output     *int `json:"output_tokens"`
	Prompt     *int `json:"prompt_tokens"`
	Completion *int `json:"completion_tokens"`
}

func (a *ResponsesUsageAccumulator) observeUsage(usage *dto.Usage, raw []byte) {
	ApplyResponsesUsage(a.usage, usage)
	if usage == nil {
		return
	}
	if usage.BillingUsage != nil && usage.BillingUsage.Estimated {
		a.upstreamEstimated = true
	}
	if len(raw) == 0 {
		// 直接构造的已有调用方继续按正数表示已报告，缺值沿用既有估算语义
		a.inputReported = a.inputReported || usage.InputTokens > 0 || usage.PromptTokens > 0
		a.outputReported = a.outputReported || usage.OutputTokens > 0 || usage.CompletionTokens > 0
		return
	}
	var document struct {
		Usage    *responsesUsageCounts `json:"usage"`
		Response struct {
			Usage *responsesUsageCounts `json:"usage"`
		} `json:"response"`
	}
	if err := common.Unmarshal(raw, &document); err != nil {
		return
	}
	counts := document.Usage
	if document.Response.Usage != nil {
		counts = document.Response.Usage
	}
	if counts == nil {
		return
	}
	input, output := counts.Input, counts.Output
	if input == nil {
		input = counts.Prompt
	}
	if output == nil {
		output = counts.Completion
	}
	if input != nil && *input >= 0 {
		a.inputReported = true
		a.usage.PromptTokens = *input
		a.usage.InputTokens = *input
	}
	if output != nil && *output >= 0 {
		a.outputReported = true
		a.usage.CompletionTokens = *output
		a.usage.OutputTokens = *output
	}
}

// responsesGeneratedText 仅汇总实际生成的正文、推理和工具参数，声明与工具结果保留原职责
func responsesGeneratedText(response *dto.OpenAIResponsesResponse) string {
	var text strings.Builder
	for _, item := range response.Output {
		var content strings.Builder
		for _, part := range item.Content {
			content.WriteString(part.Text)
		}
		text.WriteString(content.String())
		if item.Type == "reasoning" && content.Len() == 0 {
			for _, part := range item.Summary {
				text.WriteString(part.Text)
			}
		}
		if item.Type == dto.BuildInCallFunctionCall || item.Type == "mcp_call" {
			text.WriteString(item.ArgumentsString())
		}
		if item.Type == "custom_tool_call" {
			text.WriteString(dto.ResponsesArgumentsString(item.Input))
		}
	}
	return text.String()
}

func (a *ResponsesUsageAccumulator) Finish() *dto.Usage {
	if a.finished {
		return a.usage
	}
	a.finished = true
	if !a.imageCommitted {
		a.imageCounter.Commit(a.info)
		a.imageCommitted = true
	}
	estimated := a.upstreamEstimated
	completionEstimated := false
	if !a.outputReported && a.usage.CompletionTokens == 0 {
		if output := a.outputText.String(); output != "" {
			a.usage.CompletionTokens = tokenkit.Count(a.info.GetUpstreamModelName(), output)
			completionEstimated = a.usage.CompletionTokens > 0
			estimated = estimated || completionEstimated
		}
	}
	billsPrompt := a.usage.CompletionTokens != 0 || (a.started && !a.failed && !a.info.StreamStatus.ResponseFailed())
	if !a.inputReported && a.usage.PromptTokens == 0 && billsPrompt {
		a.usage.PromptTokens = a.info.GetEstimatePromptTokens()
		estimated = estimated || a.usage.PromptTokens > 0
	}
	a.usage.TotalTokens = a.usage.PromptTokens + a.usage.CompletionTokens
	if completionEstimated && a.usage.BillingUsage != nil {
		a.usage.BillingUsage = dto.CloneBillingUsageWithEstimatedCompletion(a.usage.BillingUsage, a.usage.CompletionTokens)
	}
	switch {
	case estimated:
		a.info.ResponsesUsageStatus = relaycommon.ResponsesUsageEstimated
	case a.inputReported && a.outputReported:
		a.info.ResponsesUsageStatus = relaycommon.ResponsesUsageConfirmed
	default:
		a.info.ResponsesUsageStatus = relaycommon.ResponsesUsageUnknown
	}
	return a.usage
}

func appendResponsesUsageStatus(other *model.LogOther, info *relaycommon.RelayInfo) {
	if other == nil || info == nil {
		return
	}
	status := info.ResponsesUsageStatus
	if status == "" && info.GetFinalRequestRelayFormat() != types.RelayFormatOpenAIResponses {
		return
	}
	switch status {
	case relaycommon.ResponsesUsageConfirmed, relaycommon.ResponsesUsageEstimated:
	default:
		status = relaycommon.ResponsesUsageUnknown
	}
	other.SetPublic("usage_status", status)
}

// ObserveResponsesOutcome records the protocol outcome of one Responses event
// on the stream status for health classification. Only codes and types are
// kept; messages never leave the event.
func ObserveResponsesOutcome(info *relaycommon.RelayInfo, event *dto.ResponsesStreamResponse) {
	if info == nil || info.StreamStatus == nil || event == nil {
		return
	}
	var responseStatus string
	if event.Response != nil {
		_ = common.Unmarshal(event.Response.Status, &responseStatus)
	}
	switch {
	case event.Type == "error" || event.Type == "response.failed" || event.Type == "response.error" || responseStatus == "failed":
		code, errorType := event.Code, ""
		if event.Response != nil {
			if oaiErr := event.Response.GetOpenAIError(); oaiErr != nil {
				if oaiErr.Code != nil {
					code = fmt.Sprint(oaiErr.Code)
				}
				errorType = oaiErr.Type
			}
		}
		info.StreamStatus.MarkFailed(code, errorType, 0)
	case event.Type == "response.incomplete" || responseStatus == "incomplete":
		reason := ""
		if event.Response != nil && event.Response.IncompleteDetails != nil {
			reason = event.Response.IncompleteDetails.Reason
		}
		info.StreamStatus.MarkIncomplete(reason)
	case event.Type == "response.cancelled" || event.Type == "response.canceled" || responseStatus == "cancelled":
		info.StreamStatus.MarkCancelled()
	case event.Type == "response.completed" || event.Type == "response.done" || responseStatus == "completed":
		info.StreamStatus.MarkCompleted()
	}
}

func ApplyResponsesUsage(dst *dto.Usage, src *dto.Usage) {
	if dst == nil || src == nil {
		return
	}
	incoming := relayconvert.NormalizeResponsesUsage(src)
	if src.InputTokensDetails != nil {
		inputDetails := src.InputTokensDetails.Clone()
		incoming.InputTokensDetails = &inputDetails
	}
	if src.OutputTokensDetails != nil {
		incoming.CompletionTokenDetails = *src.OutputTokensDetails
	}
	incoming.PromptCacheHitTokens = src.PromptCacheHitTokens
	dto.MergeUsageNonZero(dst, incoming)
	outputDetails := dst.CompletionTokenDetails
	if outputDetails != (dto.OutputTokenDetails{}) {
		dst.OutputTokensDetails = &outputDetails
	}
}
