package service

import (
	"context"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/subtle"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"io"
	"math"
	"net/http"
	"net/url"
	"os"
	"slices"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/dto"
	"github.com/QuantumNous/new-api/model"
	"github.com/google/uuid"
)

var (
	ErrCprMemberAccessInvalid     = errors.New("invalid member access command")
	ErrCprMemberAccessGroups      = errors.New("member access requires enabled groups")
	ErrCprMemberAccessPending     = errors.New("member access change is pending")
	ErrCprMemberAccessUnavailable = errors.New("member access is unavailable")
	memberOperations              sync.Map
)

const (
	cprMemberKeyPrefix        = "xinapi_member_"
	cprMemberKeyNamePrefix    = "xinapi-member-"
	cprMemberKeyPath          = "/api/admin/client-keys"
	cprMemberOperationTimeout = 45 * time.Second
)

func cprMemberCipher() (cipher.AEAD, error) {
	key, err := hex.DecodeString(os.Getenv("CRYPTO_SECRET"))
	if err != nil || (len(key) != 24 && len(key) != 32) {
		return nil, ErrCprMemberAccessUnavailable
	}
	defer clear(key)
	block, err := aes.NewCipher(key)
	if err != nil {
		return nil, ErrCprMemberAccessUnavailable
	}
	return cipher.NewGCM(block)
}

func cprMemberAssociatedData(access *model.CprMemberAccess) []byte {
	return []byte("xinapi-member:v1:" + strconv.Itoa(access.UserID) + ":" + strconv.FormatInt(access.Revision, 10) + ":" + access.OperationID)
}

func sealCprMemberKey(access *model.CprMemberAccess) (string, error) {
	aead, err := cprMemberCipher()
	if err != nil {
		return "", err
	}
	var secret [32]byte
	if _, err := rand.Read(secret[:]); err != nil {
		return "", ErrCprMemberAccessUnavailable
	}
	defer clear(secret[:])
	key := hex.AppendEncode([]byte(cprMemberKeyPrefix), secret[:])
	defer clear(key)
	nonce := make([]byte, aead.NonceSize())
	if _, err := rand.Read(nonce); err != nil {
		return "", ErrCprMemberAccessUnavailable
	}
	sealed := aead.Seal(nonce, nonce, key, cprMemberAssociatedData(access))
	return "v1." + base64.RawStdEncoding.EncodeToString(sealed), nil
}

func openCprMemberKey(access *model.CprMemberAccess) (string, error) {
	aead, err := cprMemberCipher()
	if err != nil {
		return "", err
	}
	encoded, ok := strings.CutPrefix(access.KeyCiphertext, "v1.")
	sealed, err := base64.RawStdEncoding.DecodeString(encoded)
	if !ok || err != nil || len(sealed) < aead.NonceSize()+aead.Overhead() {
		return "", ErrCprMemberAccessUnavailable
	}
	nonce := sealed[:aead.NonceSize()]
	key, err := aead.Open(nil, nonce, sealed[aead.NonceSize():], cprMemberAssociatedData(access))
	if err != nil {
		return "", ErrCprMemberAccessUnavailable
	}
	defer clear(key)
	if len(key) != len(cprMemberKeyPrefix)+64 || !strings.HasPrefix(string(key), cprMemberKeyPrefix) {
		return "", ErrCprMemberAccessUnavailable
	}
	return string(key), nil
}

// CprMemberCredential 解封当前成员的专属凭据，每次通过主库读取。
func CprMemberCredential(userID int) (key string, revision int64, keyID string, err error) {
	access, err := model.GetActiveCprMemberAccess(userID)
	if err != nil {
		return "", 0, "", err
	}
	key, err = openCprMemberKey(access)
	if err != nil {
		return "", 0, "", err
	}
	return key, access.Revision, access.CprKeyID, nil
}

func cprMemberView(access *model.CprMemberAccess) (dto.CprMemberAccess, error) {
	if access == nil {
		return dto.CprMemberAccess{}, ErrCprMemberAccessUnavailable
	}
	groups, err := access.GroupIDs()
	if err != nil {
		return dto.CprMemberAccess{}, ErrCprMemberAccessUnavailable
	}
	return dto.CprMemberAccess{UserID: access.UserID, Revision: access.Revision, GroupIDs: groups,
		Status: access.Status, Stage: access.Stage, OperationID: access.OperationID,
		CprKeyID: access.CprKeyID, UpdatedAt: access.UpdatedAt}, nil
}

func GetCprMemberAccess(ctx context.Context, userID int) (dto.CprMemberAccess, error) {
	access, err := model.GetCprMemberAccess(ctx, userID)
	if err != nil {
		return dto.CprMemberAccess{}, err
	}
	return cprMemberView(access)
}

func cprMemberGroupIDs(groups []string) ([]string, error) {
	if groups == nil || len(groups) > 1000 {
		return nil, ErrCprMemberAccessInvalid
	}
	normalized := slices.Clone(groups)
	slices.Sort(normalized)
	for i, group := range normalized {
		suffix, ok := strings.CutPrefix(group, "grp_")
		if !ok || len(suffix) != 32 || i > 0 && normalized[i-1] == group {
			return nil, ErrCprMemberAccessInvalid
		}
		for j := range len(suffix) {
			if b := suffix[j]; (b < '0' || b > '9') && (b < 'a' || b > 'f') {
				return nil, ErrCprMemberAccessInvalid
			}
		}
	}
	return normalized, nil
}

func requireCprMemberGroups(ctx context.Context, groups []string) error {
	if len(groups) == 0 {
		return nil
	}
	remaining := make(map[string]bool, len(groups))
	for _, group := range groups {
		remaining[group] = true
	}
	for page := 1; page <= 100; page++ {
		query := url.Values{"page": {strconv.Itoa(page)}, "pageSize": {"50"}, "enabled": {"true"}}
		result, err := cprManagementData[dto.AccountEgressPage[dto.AccountEgressGroup]](ctx, http.MethodGet,
			"/api/admin/account-groups/options", query, nil, cprManagementReadTimeout)
		if err != nil {
			return ErrCprMemberAccessUnavailable
		}
		for _, group := range result.Items {
			if group.Enabled {
				delete(remaining, group.ID)
			}
		}
		if len(remaining) == 0 {
			return nil
		}
		if page >= result.Page.TotalPages {
			return ErrCprMemberAccessGroups
		}
	}
	return ErrCprMemberAccessUnavailable
}

func SaveCprMemberAccess(ctx context.Context, identity model.AuthSessionIdentity, userID int, cmd dto.CprMemberGroupsCommand, authorize model.CprMemberAccessAuthorizer) (dto.CprMemberAccess, error) {
	if userID <= 0 || cmd.ExpectedRevision == nil || *cmd.ExpectedRevision < 0 || *cmd.ExpectedRevision == math.MaxInt64 {
		return dto.CprMemberAccess{}, ErrCprMemberAccessInvalid
	}
	groups, err := cprMemberGroupIDs(cmd.GroupIDs)
	if err != nil {
		return dto.CprMemberAccess{}, err
	}
	if _, busy := memberOperations.LoadOrStore(userID, struct{}{}); busy {
		return dto.CprMemberAccess{}, model.ErrCprMemberAccessConflict
	}
	defer memberOperations.Delete(userID)
	ctx, cancel := context.WithTimeout(ctx, cprMemberOperationTimeout)
	defer cancel()
	current, err := model.GetCprMemberAccess(ctx, userID)
	if err != nil {
		return dto.CprMemberAccess{}, err
	}
	if current.Revision != *cmd.ExpectedRevision || (current.Status != model.CprMemberAccessActive && current.Status != model.CprMemberAccessRevoked) {
		return dto.CprMemberAccess{}, model.ErrCprMemberAccessConflict
	}
	if current.Status == model.CprMemberAccessRevoked && len(groups) == 0 {
		if err := model.CheckCprMemberAccessManagement(ctx, identity, userID, authorize); err != nil {
			return dto.CprMemberAccess{}, err
		}
		return cprMemberView(current)
	}
	if err := requireCprMemberGroups(ctx, groups); err != nil {
		return dto.CprMemberAccess{}, err
	}
	encoded, err := common.Marshal(groups)
	if err != nil {
		return dto.CprMemberAccess{}, ErrCprMemberAccessUnavailable
	}
	operation, err := uuid.NewRandom()
	if err != nil {
		return dto.CprMemberAccess{}, ErrCprMemberAccessUnavailable
	}
	next := &model.CprMemberAccess{UserID: userID, Revision: current.Revision + 1, GroupIDsJSON: string(encoded), Status: model.CprMemberAccessPending,
		Stage: model.CprMemberCreateKey, OperationID: operation.String(), UpdatedAt: time.Now().Unix()}
	if current.Status == model.CprMemberAccessActive {
		if current.CprKeyID == "" || current.OperationID == "" {
			return dto.CprMemberAccess{}, ErrCprMemberAccessUnavailable
		}
		next.Stage, next.PreviousKeyID, next.PreviousOperationID = model.CprMemberDisableOld, current.CprKeyID, current.OperationID
	}
	if len(groups) > 0 {
		next.KeyCiphertext, err = sealCprMemberKey(next)
		if err != nil {
			return dto.CprMemberAccess{}, err
		}
	}
	if err := model.BeginCprMemberAccess(ctx, identity, *cmd.ExpectedRevision, next, authorize); err != nil {
		return dto.CprMemberAccess{}, err
	}
	return runCprMemberAccess(ctx, identity, next, groups, authorize, false)
}

func ReconcileCprMemberAccess(ctx context.Context, identity model.AuthSessionIdentity, userID int, cmd dto.CprMemberReconcileCommand, authorize model.CprMemberAccessAuthorizer) (dto.CprMemberAccess, error) {
	operation, err := uuid.Parse(cmd.OperationID)
	if userID <= 0 || cmd.ExpectedRevision == nil || *cmd.ExpectedRevision <= 0 || err != nil || operation == uuid.Nil {
		return dto.CprMemberAccess{}, ErrCprMemberAccessInvalid
	}
	if _, busy := memberOperations.LoadOrStore(userID, struct{}{}); busy {
		return dto.CprMemberAccess{}, model.ErrCprMemberAccessConflict
	}
	defer memberOperations.Delete(userID)
	ctx, cancel := context.WithTimeout(ctx, cprMemberOperationTimeout)
	defer cancel()
	access, err := model.GetCprMemberAccess(ctx, userID)
	if err != nil {
		return dto.CprMemberAccess{}, err
	}
	if access.OperationID != operation.String() || access.Revision != *cmd.ExpectedRevision {
		return dto.CprMemberAccess{}, model.ErrCprMemberAccessConflict
	}
	if access.Status == model.CprMemberAccessActive || access.Status == model.CprMemberAccessRevoked {
		if cmd.Revoke && access.Status == model.CprMemberAccessActive {
			return dto.CprMemberAccess{}, model.ErrCprMemberAccessConflict
		}
		if err := model.CheckCprMemberAccessManagement(ctx, identity, userID, authorize); err != nil {
			return dto.CprMemberAccess{}, err
		}
		return cprMemberView(access)
	}
	if cmd.Revoke && access.Stage != model.CprMemberRevokeKeys {
		if err := model.BeginCprMemberRevocation(ctx, identity, access, authorize); err != nil {
			return dto.CprMemberAccess{}, err
		}
	}
	groups, err := access.GroupIDs()
	if err != nil {
		return dto.CprMemberAccess{}, ErrCprMemberAccessUnavailable
	}
	return runCprMemberAccess(ctx, identity, access, groups, authorize, true)
}

// runCprMemberAccess 以持久领取执行远端调用，含糊结果保持停发并由管理员读回。
func runCprMemberAccess(ctx context.Context, identity model.AuthSessionIdentity, access *model.CprMemberAccess, groups []string, authorize model.CprMemberAccessAuthorizer, reconcile bool) (dto.CprMemberAccess, error) {
	for access.Stage == model.CprMemberDisableOld || access.Stage == model.CprMemberCreateKey || access.Stage == model.CprMemberRevokeKeys {
		previouslyAttempted := access.StageAttempt > 0
		if err := model.ClaimCprMemberAccessStage(ctx, identity, access, time.Now().Add(2*time.Minute).Unix(), authorize); err != nil {
			view, _ := cprMemberView(access)
			return view, err
		}
		status, stage, keyID := model.CprMemberAccessUnknown, access.Stage, ""
		switch access.Stage {
		case model.CprMemberDisableOld:
			if disableCprMemberKey(ctx, access, reconcile && previouslyAttempted) {
				if len(groups) == 0 {
					status, stage = model.CprMemberAccessRevoked, ""
				} else {
					status, stage = model.CprMemberAccessPending, model.CprMemberCreateKey
				}
			}
		case model.CprMemberCreateKey:
			if len(groups) > 0 {
				if id := createCprMemberKey(ctx, access, groups, reconcile && previouslyAttempted); id != "" {
					status, stage, keyID = model.CprMemberAccessActive, "", id
				}
			}
		case model.CprMemberRevokeKeys:
			if revokeCprMemberKeys(ctx, access) {
				status, stage = model.CprMemberAccessRevoked, ""
			}
		}
		saveCtx, cancel := context.WithTimeout(context.WithoutCancel(ctx), 5*time.Second)
		err := model.FinishCprMemberAccessStage(saveCtx, access, status, stage, keyID)
		cancel()
		if err != nil {
			view, _ := cprMemberView(access)
			return view, err
		}
		if status != model.CprMemberAccessPending {
			view, err := cprMemberView(access)
			if err == nil && status == model.CprMemberAccessUnknown {
				err = ErrCprMemberAccessPending
			}
			return view, err
		}
	}
	return dto.CprMemberAccess{}, ErrCprMemberAccessUnavailable
}

type cprMemberKeyProfile struct {
	OpenAI struct {
		Inherit struct {
			InstallationSeed string `json:"installationSeed"`
		} `json:"inherit"`
	} `json:"openai"`
}

type cprMemberKeyRecord struct {
	ID           string                   `json:"id"`
	Name         string                   `json:"name"`
	Enabled      *bool                    `json:"enabled"`
	RoutingScope string                   `json:"routingScope"`
	Groups       []dto.AccountEgressGroup `json:"groups"`
	Profile      cprMemberKeyProfile      `json:"providerRequestProfileOverrides"`
}

type cprMemberKeySecret struct {
	ID        string `json:"id"`
	Plaintext string `json:"plaintextKey"`
}

func findCprMemberKey(ctx context.Context, operationID, keyID string) (*cprMemberKeyRecord, error) {
	name := cprMemberKeyNamePrefix + operationID
	query := url.Values{"search": {name}, "limit": {"50"}}
	var match *cprMemberKeyRecord
	for range 4 {
		result, err := cprManagementData[struct {
			Items      []cprMemberKeyRecord `json:"items"`
			NextCursor string               `json:"nextCursor"`
		}](ctx, http.MethodGet, cprMemberKeyPath, query, nil, cprManagementReadTimeout)
		if err != nil {
			return nil, ErrCprMemberAccessUnavailable
		}
		for _, item := range result.Items {
			if item.Name == name && (keyID == "" || item.ID == keyID) {
				if match != nil || item.ID == "" || item.Enabled == nil {
					return nil, ErrCprMemberAccessUnavailable
				}
				match = &item
			}
		}
		if result.NextCursor == "" {
			return match, nil
		}
		query.Set("cursor", result.NextCursor)
	}
	return nil, ErrCprMemberAccessUnavailable
}

func revealCprMemberKey(ctx context.Context, keyID string) (cprMemberKeySecret, error) {
	return cprManagementData[cprMemberKeySecret](ctx, http.MethodGet, cprMemberKeyPath+"/reveal", url.Values{"id": {keyID}}, nil, cprManagementReadTimeout)
}

func disableCprMemberKey(ctx context.Context, access *model.CprMemberAccess, readback bool) bool {
	if access.PreviousKeyID == "" {
		return false
	}
	if readback {
		key, err := findCprMemberKey(ctx, access.PreviousOperationID, access.PreviousKeyID)
		if err != nil {
			return false
		}
		if key != nil && !*key.Enabled {
			return true
		}
		if key == nil {
			_, err := revealCprMemberKey(ctx, access.PreviousKeyID)
			var notFound *CPRManagementError
			if errors.As(err, &notFound) && notFound.Status == http.StatusNotFound {
				return true
			}
			if err != nil {
				return false
			}
		}
	}
	return postDisableCprMemberKey(ctx, access.PreviousKeyID)
}

func postDisableCprMemberKey(ctx context.Context, keyID string) bool {
	type keyRef struct {
		ID string `json:"id"`
	}
	result, err := cprManagementData[keyRef](ctx, http.MethodPost, cprMemberKeyPath+"/disable", nil,
		keyRef{ID: keyID}, cprManagementWriteTimeout)
	return err == nil && result.ID == keyID
}

// revokeCprMemberKeys 先收尾旧 Key，再核对同操作可能已经创建的 Key。
func revokeCprMemberKeys(ctx context.Context, access *model.CprMemberAccess) bool {
	if access.PreviousKeyID != "" && !disableCprMemberKey(ctx, access, true) {
		return false
	}
	key, err := findCprMemberKey(ctx, access.OperationID, "")
	if err != nil {
		return false
	}
	if key == nil {
		// 已发送的创建可能仍在生效。没有远端证据时保留同一操作等待核对。
		return !access.CreateIssued
	}
	plaintext, err := openCprMemberKey(access)
	if err != nil {
		return false
	}
	revealed, err := revealCprMemberKey(ctx, key.ID)
	if err != nil || revealed.ID != key.ID || subtle.ConstantTimeCompare([]byte(plaintext), []byte(revealed.Plaintext)) != 1 {
		return false
	}
	return !*key.Enabled || postDisableCprMemberKey(ctx, key.ID)
}

func createCprMemberKey(ctx context.Context, access *model.CprMemberAccess, groups []string, readback bool) string {
	key, err := openCprMemberKey(access)
	if err != nil || len(groups) == 0 {
		return ""
	}
	if readback {
		existing, err := findCprMemberKey(ctx, access.OperationID, "")
		if err != nil {
			return ""
		}
		if existing != nil {
			if !*existing.Enabled || existing.RoutingScope != "groups" || existing.Profile.OpenAI.Inherit.InstallationSeed != access.OperationID {
				return ""
			}
			ids := make([]string, 0, len(existing.Groups))
			for _, group := range existing.Groups {
				if !group.Enabled {
					return ""
				}
				ids = append(ids, group.ID)
			}
			slices.Sort(ids)
			if !slices.Equal(ids, groups) {
				return ""
			}
			revealed, err := revealCprMemberKey(ctx, existing.ID)
			if err != nil || revealed.ID != existing.ID || subtle.ConstantTimeCompare([]byte(key), []byte(revealed.Plaintext)) != 1 {
				return ""
			}
			return existing.ID
		}
	}
	if err := requireCprMemberGroups(ctx, groups); err != nil {
		return ""
	}
	var profile cprMemberKeyProfile
	profile.OpenAI.Inherit.InstallationSeed = access.OperationID
	command := struct {
		Name              string              `json:"name"`
		CustomKey         string              `json:"customKey"`
		GroupIDs          []string            `json:"groupIds"`
		MaxConcurrency    int                 `json:"maxConcurrency"`
		RequestsPerMinute int                 `json:"requestsPerMinute"`
		Profile           cprMemberKeyProfile `json:"providerRequestProfileOverrides"`
	}{Name: cprMemberKeyNamePrefix + access.OperationID, CustomKey: key, GroupIDs: groups, Profile: profile}
	ctx, cancel := context.WithTimeout(ctx, cprManagementWriteTimeout)
	defer cancel()
	if err := model.MarkCprMemberCreateIssued(ctx, access); err != nil {
		return ""
	}
	response, err := sendCPRAdmin(ctx, cprMutationClient, http.MethodPost, cprMemberKeyPath+"/create", nil, command)
	if err != nil {
		return ""
	}
	defer response.Body.Close()
	if response.StatusCode != http.StatusCreated {
		return ""
	}
	var envelope struct {
		Code int                `json:"code"`
		Data cprMemberKeySecret `json:"data"`
	}
	body := &io.LimitedReader{R: response.Body, N: 16 << 10}
	if err := common.DecodeJsonBody(body, &envelope); err != nil || body.N == 0 || envelope.Code != http.StatusOK || envelope.Data.ID == "" ||
		subtle.ConstantTimeCompare([]byte(key), []byte(envelope.Data.Plaintext)) != 1 {
		return ""
	}
	return envelope.Data.ID
}
