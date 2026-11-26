package model

import (
	"errors"
	"time"

	"github.com/QuantumNous/new-api/common"
	"gorm.io/gorm"
)

var (
	ErrCprOAuthMissing   = errors.New("account authorization operation is missing")
	ErrCprOAuthOwner     = errors.New("account authorization operation belongs to another session")
	ErrCprOAuthConflict  = errors.New("account authorization operation changed")
	ErrCprOAuthExpired   = errors.New("account authorization operation has expired")
	ErrCprOAuthForbidden = errors.New("account authorization permission is required")
)

// CprOAuthOperation只保存操作归属与脱敏回执，授权材料始终由CPR持有
// AttemptAt一经领取即保留，响应丢失和服务重启后通过只读回执确认结果
// 时间使用UTC Unix秒，保持三种数据库的列类型与条件写入语义一致
// Revision仅描述本地操作记录，与CPR路由代次分别核验
type CprOAuthOperation struct {
	OperationID     string `gorm:"type:varchar(36);primaryKey"`
	UserID          int    `gorm:"index:idx_cpr_oauth_owner"`
	SessionID       string `gorm:"type:varchar(64);index:idx_cpr_oauth_owner"`
	UserAuthVersion int64
	SessionVersion  int64
	Provider        string `gorm:"type:varchar(16)"`
	Name            string `gorm:"type:varchar(100)"`
	TargetAccountID string `gorm:"type:varchar(256)"`
	AccountID       string `gorm:"type:varchar(256)"`
	SourceID        string `gorm:"type:varchar(36)"`
	NodeID          string `gorm:"type:varchar(36)"`
	VersionID       string `gorm:"type:varchar(36)"`
	ProxyID         string `gorm:"type:varchar(256)"`
	RouteGeneration int64
	ExpiresAt       int64  `gorm:"index"`
	Status          string `gorm:"type:varchar(16)"`
	Reason          string `gorm:"type:varchar(64)"`
	AttemptKind     string `gorm:"type:varchar(16)"`
	AttemptAt       int64
	CancelAttemptAt int64 `gorm:"not null;default:0"`
	CreatedAt       int64
	UpdatedAt       int64
	Revision        int64
}

// CprOAuthAuthorizer在事务中读取当前角色后核对操作权限
type CprOAuthAuthorizer func(userID, role int) bool

func validateCprOAuthSession(tx *gorm.DB, identity AuthSessionIdentity, authorize CprOAuthAuthorizer) error {
	if _, token := ParseAccessTokenSessionID(identity.SessionID); token {
		return ErrUserSessionInactive
	}
	if err := ValidateAuthSessionWithTx(tx, identity); err != nil {
		return err
	}
	var user User
	if err := tx.Select("id", "role").First(&user, identity.UserID).Error; err != nil {
		return err
	}
	if user.Role < common.RoleAdminUser || authorize == nil || !authorize(user.Id, user.Role) {
		return ErrCprOAuthForbidden
	}
	return nil
}

func cprOAuthOwned(operation *CprOAuthOperation, identity AuthSessionIdentity) bool {
	return operation.UserID == identity.UserID && operation.SessionID == identity.SessionID && operation.UserAuthVersion == identity.UserAuthVersion && operation.SessionVersion == identity.SessionVersion
}

func CreateCprOAuthOperation(identity AuthSessionIdentity, operation *CprOAuthOperation, authorize CprOAuthAuthorizer) error {
	return DB.Transaction(func(tx *gorm.DB) error {
		// 先写新操作再读取会话，SQLite也由第一笔写入确定并发顺序
		if err := tx.Create(operation).Error; err != nil {
			return err
		}
		if !cprOAuthOwned(operation, identity) {
			return ErrCprOAuthOwner
		}
		return validateCprOAuthSession(tx, identity, authorize)
	})
}

func GetCprOAuthOperation(identity AuthSessionIdentity, id string, authorize CprOAuthAuthorizer) (*CprOAuthOperation, error) {
	var operation CprOAuthOperation
	err := DB.Transaction(func(tx *gorm.DB) error {
		if err := validateCprOAuthSession(tx, identity, authorize); err != nil {
			return err
		}
		if err := tx.Where("operation_id = ?", id).First(&operation).Error; err != nil {
			if errors.Is(err, gorm.ErrRecordNotFound) {
				return ErrCprOAuthMissing
			}
			return err
		}
		if !cprOAuthOwned(&operation, identity) {
			return ErrCprOAuthOwner
		}
		return nil
	})
	if err != nil {
		return nil, err
	}
	return &operation, nil
}

func ListCprOAuthOperations(identity AuthSessionIdentity, authorize CprOAuthAuthorizer) ([]CprOAuthOperation, error) {
	operations := make([]CprOAuthOperation, 0)
	err := DB.Transaction(func(tx *gorm.DB) error {
		if err := validateCprOAuthSession(tx, identity, authorize); err != nil {
			return err
		}
		return tx.Where("user_id = ? AND session_id = ? AND user_auth_version = ? AND session_version = ?", identity.UserID, identity.SessionID, identity.UserAuthVersion, identity.SessionVersion).Order("created_at DESC").Order("operation_id DESC").Limit(100).Find(&operations).Error
	})
	return operations, err
}

// ClaimCprOAuthOperation以短事务领取发送；complete限定一次，cancel允许显式重复资源收尾
func ClaimCprOAuthOperation(identity AuthSessionIdentity, id string, revision int64, kind string, authorize CprOAuthAuthorizer) (*CprOAuthOperation, error) {
	if kind != "complete" && kind != "cancel" {
		return nil, ErrCprOAuthConflict
	}
	var operation CprOAuthOperation
	err := DB.Transaction(func(tx *gorm.DB) error {
		now := time.Now().Unix()
		query := tx.Model(&CprOAuthOperation{}).
			Where("operation_id = ? AND user_id = ? AND session_id = ? AND user_auth_version = ? AND session_version = ? AND revision = ?", id, identity.UserID, identity.SessionID, identity.UserAuthVersion, identity.SessionVersion, revision)
		updates := map[string]any{"status": "completing", "attempt_kind": kind, "updated_at": now, "revision": revision + 1}
		if kind == "cancel" {
			query = query.Where("status IN ?", []string{"pending", "unknown", "failed", "completing", "cancelled"})
			updates["cancel_attempt_at"] = gorm.Expr("CASE WHEN cancel_attempt_at = 0 THEN ? ELSE cancel_attempt_at END", now)
		} else {
			query = query.Where("attempt_at = 0 AND cancel_attempt_at = 0 AND status = ? AND expires_at > ?", "pending", now)
			updates["attempt_at"] = now
		}
		result := query.Updates(updates)
		if result.Error != nil {
			return result.Error
		}
		if result.RowsAffected != 1 {
			return ErrCprOAuthConflict
		}
		if err := validateCprOAuthSession(tx, identity, authorize); err != nil {
			return err
		}
		return tx.Where("operation_id = ?", id).First(&operation).Error
	})
	if err != nil {
		return nil, err
	}
	return &operation, nil
}

// SaveCprOAuthReceipt仅接受当前revision，迟到回执保留较新的状态
func SaveCprOAuthReceipt(operation *CprOAuthOperation) (bool, error) {
	result := DB.Model(&CprOAuthOperation{}).Where("operation_id = ? AND revision = ?", operation.OperationID, operation.Revision).
		Updates(map[string]any{"account_id": operation.AccountID, "source_id": operation.SourceID, "node_id": operation.NodeID, "version_id": operation.VersionID, "proxy_id": operation.ProxyID, "route_generation": operation.RouteGeneration, "expires_at": operation.ExpiresAt, "status": operation.Status, "reason": operation.Reason, "updated_at": time.Now().Unix(), "revision": operation.Revision + 1})
	if result.Error != nil {
		return false, result.Error
	}
	if result.RowsAffected == 1 {
		operation.Revision++
		operation.UpdatedAt = time.Now().Unix()
		return true, nil
	}
	return false, nil
}
