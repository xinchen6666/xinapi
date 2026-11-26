package service

import (
	"errors"
	"net/http"
	"net/url"
	"os"
	"strconv"
	"strings"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/constant"
	"github.com/QuantumNous/new-api/model"
	"github.com/QuantumNous/new-api/relaykit/dto"
	"github.com/QuantumNous/new-api/relaykit/types"
	"github.com/gin-gonic/gin"
)

const (
	cprMemberRelayHost        = "cpr:8080"
	CprMemberRelayOrigin      = "http://" + cprMemberRelayHost
	CprMemberRelayPlaceholder = "xinapi-member-managed"
	cprMemberRelayBindingKey  = "xinapi.cpr_member_relay.binding"
)

// CprMemberBinding 保存请求与连接使用的成员授权标识，凭据仅在发送时解封
type CprMemberBinding struct {
	MemberID int
	Revision int64
	KeyID    string
}

func CprMemberRelayBinding(c *gin.Context) (CprMemberBinding, bool) {
	if c == nil {
		return CprMemberBinding{}, false
	}
	value, _ := c.Get(cprMemberRelayBindingKey)
	binding, ok := value.(CprMemberBinding)
	return binding, ok && binding.MemberID > 0
}

// IsCprMemberChannel 按服务端配置识别专用渠道
func IsCprMemberChannel(channelID int) bool {
	configured, err := strconv.Atoi(os.Getenv("XINAPI_CPR_CHANNEL_ID"))
	return err == nil && configured > 0 && channelID == configured
}

// IsCprMemberRelayTarget 将 CPR 内部服务名保留给成员通道
func IsCprMemberRelayTarget(target *url.URL) bool {
	return target != nil && strings.EqualFold(strings.TrimSuffix(target.Hostname(), "."), "cpr")
}

func cprMemberRelayError(message string, status int) *types.NewAPIError {
	return types.NewErrorWithStatusCode(errors.New(message), types.ErrorCodeAccessDenied, status, types.ErrOptionWithSkipRetry())
}

// CheckCprMemberChannel 核对固定目标与原生适配器
func CheckCprMemberChannel(channel *model.Channel) (bool, *types.NewAPIError) {
	managed := IsCprMemberChannel(channel.Id)
	base := channel.GetBaseURL()
	if !managed {
		if target, err := url.Parse(base); err == nil && IsCprMemberRelayTarget(target) {
			return false, cprMemberRelayError("The internal gateway requires its dedicated member channel.", http.StatusForbidden)
		}
		return false, nil
	}
	if channel.Type != constant.ChannelTypeOpenAI || base != CprMemberRelayOrigin || !validCprMemberChannelSettings(channel) {
		return false, cprMemberRelayError("The internal member channel configuration is invalid.", http.StatusForbidden)
	}
	return true, nil
}

// validCprMemberChannelSettings 解析原配置，拒绝格式错误及外部传输覆盖
func validCprMemberChannelSettings(channel *model.Channel) bool {
	var settings dto.ChannelSettings
	if channel.Setting != nil && *channel.Setting != "" {
		if err := common.UnmarshalJsonStr(*channel.Setting, &settings); err != nil || settings.Proxy != "" {
			return false
		}
	}
	var other dto.ChannelOtherSettings
	if channel.OtherSettings != "" {
		if err := common.UnmarshalJsonStr(channel.OtherSettings, &other); err != nil || other.AdvancedCustom != nil {
			return false
		}
	}
	if channel.HeaderOverride != nil && *channel.HeaderOverride != "" {
		var headers map[string]any
		if err := common.UnmarshalJsonStr(*channel.HeaderOverride, &headers); err != nil || len(headers) > 0 {
			return false
		}
	}
	return true
}

// SetupCprMemberRelay 在适配器处理前绑定当前成员与授权代次
func SetupCprMemberRelay(c *gin.Context, channel *model.Channel) (bool, *types.NewAPIError) {
	c.Set(cprMemberRelayBindingKey, CprMemberBinding{})
	managed, apiErr := CheckCprMemberChannel(channel)
	if apiErr != nil || !managed {
		return managed, apiErr
	}
	if c.Request == nil || c.Request.URL == nil || !cprMemberHTTPPath(c.Request.Method, c.Request.URL.Path) {
		return true, cprMemberRelayError("The internal member channel accepts Responses and Models only.", http.StatusForbidden)
	}
	memberID := c.GetInt("id")
	if memberID <= 0 || c.GetInt("token_id") <= 0 {
		return true, cprMemberRelayError("A valid member API key is required for the internal gateway.", http.StatusForbidden)
	}
	access, err := model.GetActiveCprMemberAccess(memberID)
	if err != nil || access == nil || access.Revision <= 0 || access.CprKeyID == "" {
		return true, deniedCprMemberRelay()
	}
	c.Set(cprMemberRelayBindingKey, CprMemberBinding{MemberID: memberID, Revision: access.Revision, KeyID: access.CprKeyID})
	return true, nil
}

func deniedCprMemberRelay() *types.NewAPIError {
	return cprMemberRelayError("Member access to the internal gateway is unavailable.", http.StatusForbidden)
}

func cprMemberHTTPPath(method string, path string) bool {
	return method == http.MethodPost && path == "/v1/responses" || method == http.MethodGet && path == "/v1/models"
}

// AuthorizeCprMemberRelayTarget 核对最终目标并仅为当次发送解封凭据
func AuthorizeCprMemberRelayTarget(c *gin.Context, channelID int, method string, target *url.URL, websocket bool) (string, *types.NewAPIError) {
	binding, bound := CprMemberRelayBinding(c)
	if !IsCprMemberChannel(channelID) || !bound {
		return "", deniedCprMemberRelay()
	}
	scheme := "http"
	pathAllowed := target != nil && cprMemberHTTPPath(method, target.Path)
	if websocket {
		scheme = "ws"
		pathAllowed = target != nil && method == http.MethodGet && target.Path == "/v1/responses"
	}
	if !pathAllowed || target.Scheme != scheme || target.Host != cprMemberRelayHost ||
		target.User != nil || target.Opaque != "" || target.RawPath != "" || target.RawQuery != "" || target.ForceQuery || target.Fragment != "" {
		return "", cprMemberRelayError("The internal gateway target is outside the member relay contract.", http.StatusForbidden)
	}
	return currentCprMemberCredential(c, binding)
}

// RecheckCprMemberBinding 核对已建连接与当前授权的归属和代次
func RecheckCprMemberBinding(c *gin.Context, binding CprMemberBinding) *types.NewAPIError {
	if c.Request.Context().Err() != nil || c.GetInt("id") != binding.MemberID || c.GetInt("token_id") <= 0 {
		return deniedCprMemberRelay()
	}
	access, err := model.GetActiveCprMemberAccess(binding.MemberID)
	if err != nil || access == nil || access.Revision != binding.Revision || access.CprKeyID != binding.KeyID {
		return deniedCprMemberRelay()
	}
	return nil
}

func currentCprMemberCredential(c *gin.Context, binding CprMemberBinding) (string, *types.NewAPIError) {
	if c.Request.Context().Err() != nil || c.GetInt("id") != binding.MemberID || c.GetInt("token_id") <= 0 {
		return "", deniedCprMemberRelay()
	}
	key, revision, keyID, err := CprMemberCredential(binding.MemberID)
	if err != nil || revision != binding.Revision || keyID != binding.KeyID || key == "" || strings.ContainsAny(key, " \t\r\n") {
		return "", deniedCprMemberRelay()
	}
	return key, nil
}
