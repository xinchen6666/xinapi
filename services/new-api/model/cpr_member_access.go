package model

import (
	"context"
	"errors"
	"math"
	"time"

	"github.com/QuantumNous/new-api/common"
	"gorm.io/gorm"
	"gorm.io/gorm/clause"
	"gorm.io/gorm/logger"
)

const (
	CprMemberAccessRevoked = "revoked"
	CprMemberAccessPending = "pending"
	CprMemberAccessUnknown = "unknown"
	CprMemberAccessActive  = "active"
	CprMemberDisableOld    = "disable_old"
	CprMemberCreateKey     = "create_key"
	CprMemberRevokeKeys    = "revoke_keys"
)

var (
	ErrCprMemberAccessDenied    = errors.New("CPR member access is inactive")
	ErrCprMemberAccessConflict  = errors.New("CPR member access changed")
	ErrCprMemberAccessForbidden = errors.New("CPR member access management is forbidden")
	ErrCprMemberUserDisabled    = errors.New("CPR member user is disabled")
)

// CprMemberAccess 保存单成员授权、当前操作及远端 Key 的恢复信息。
// 管理 API 通过独立 DTO 投影安全字段；密文与领取字段仅留服务端。
type CprMemberAccess struct {
	UserID              int    `gorm:"primaryKey;autoIncrement:false" json:"-"`
	Revision            int64  `gorm:"not null" json:"-"`
	GroupIDsJSON        string `gorm:"column:group_ids;type:text;not null" json:"-"`
	Status              string `gorm:"type:varchar(16);not null" json:"-"`
	Stage               string `gorm:"type:varchar(16);not null" json:"-"`
	OperationID         string `gorm:"type:varchar(36);not null" json:"-"`
	CprKeyID            string `gorm:"size:128;not null" json:"-"`
	KeyCiphertext       string `gorm:"type:text;not null" json:"-"`
	PreviousKeyID       string `gorm:"size:128;not null" json:"-"`
	PreviousOperationID string `gorm:"type:varchar(36);not null" json:"-"`
	CreateIssued        bool   `gorm:"not null;default:false" json:"-"`
	StageAttempt        int64  `gorm:"not null" json:"-"`
	AttemptUntil        int64  `gorm:"not null" json:"-"`
	UpdatedAt           int64  `json:"-"`
}

func (CprMemberAccess) TableName() string { return "cpr_member_accesses" }

func (access CprMemberAccess) GroupIDs() ([]string, error) {
	var groups []string
	if err := common.UnmarshalJsonStr(access.GroupIDsJSON, &groups); err != nil || groups == nil {
		return nil, ErrCprMemberAccessDenied
	}
	return groups, nil
}

func memberAccessDB(ctx context.Context) *gorm.DB {
	return DB.WithContext(ctx).Session(&gorm.Session{Logger: logger.Discard})
}

func GetCprMemberAccess(ctx context.Context, userID int) (*CprMemberAccess, error) {
	var access CprMemberAccess
	err := memberAccessDB(ctx).First(&access, "user_id = ?", userID).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return &CprMemberAccess{UserID: userID, GroupIDsJSON: "[]", Status: CprMemberAccessRevoked}, nil
	}
	if err != nil {
		return nil, err
	}
	return &access, nil
}

// GetActiveCprMemberAccess 从主库同时核对当前用户与授权状态。
func GetActiveCprMemberAccess(userID int) (*CprMemberAccess, error) {
	if userID <= 0 {
		return nil, ErrCprMemberAccessDenied
	}
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	db := memberAccessDB(ctx)
	user := db.Session(&gorm.Session{NewDB: true}).Model(&User{}).Select("1").Where("users.id = cpr_member_accesses.user_id AND users.status = ?", common.UserStatusEnabled)
	var access CprMemberAccess
	err := db.Where("user_id = ? AND status = ? AND group_ids <> ?", userID, CprMemberAccessActive, "[]").Where("EXISTS (?)", user).First(&access).Error
	if errors.Is(err, gorm.ErrRecordNotFound) {
		return nil, ErrCprMemberAccessDenied
	}
	if err != nil {
		return nil, err
	}
	groups, err := access.GroupIDs()
	if err != nil || len(groups) == 0 || access.CprKeyID == "" || access.KeyCiphertext == "" {
		return nil, ErrCprMemberAccessDenied
	}
	return &access, nil
}

type CprMemberAccessAuthorizer func(actorID, actorRole, targetRole int) bool

func authorizeCprMemberAccess(tx *gorm.DB, identity AuthSessionIdentity, userID int, requireEnabled bool, authorize CprMemberAccessAuthorizer) error {
	if _, token := ParseAccessTokenSessionID(identity.SessionID); token {
		return ErrUserSessionInactive
	}
	if err := ValidateAuthSessionWithTx(tx, identity); err != nil {
		return err
	}
	var actor, target User
	if err := tx.Select("id", "role").First(&actor, "id = ?", identity.UserID).Error; err != nil {
		return err
	}
	if err := tx.Select("id", "role", "status").First(&target, "id = ?", userID).Error; err != nil {
		return err
	}
	if authorize == nil || !authorize(actor.Id, actor.Role, target.Role) {
		return ErrCprMemberAccessForbidden
	}
	if requireEnabled && target.Status != common.UserStatusEnabled {
		return ErrCprMemberUserDisabled
	}
	return nil
}

// BeginCprMemberAccess 冻结本次目标，先使既有授权停发，远端操作随后执行。
func BeginCprMemberAccess(ctx context.Context, identity AuthSessionIdentity, expected int64, next *CprMemberAccess, authorize CprMemberAccessAuthorizer) error {
	if expected < 0 || expected == math.MaxInt64 || next.Revision != expected+1 || next.Status != CprMemberAccessPending {
		return ErrCprMemberAccessConflict
	}
	return memberAccessDB(ctx).Transaction(func(tx *gorm.DB) error {
		if expected == 0 {
			result := tx.Clauses(clause.OnConflict{Columns: []clause.Column{{Name: "user_id"}}, DoNothing: true}).Create(next)
			if result.Error != nil {
				return result.Error
			}
			var current CprMemberAccess
			if err := tx.Select("operation_id").First(&current, "user_id = ?", next.UserID).Error; err != nil {
				return err
			}
			if current.OperationID != next.OperationID {
				return ErrCprMemberAccessConflict
			}
		} else {
			result := tx.Model(&CprMemberAccess{}).
				Where("user_id = ? AND revision = ? AND status IN ?", next.UserID, expected, []string{CprMemberAccessRevoked, CprMemberAccessActive}).
				Updates(map[string]any{
					"revision": next.Revision, "group_ids": next.GroupIDsJSON, "status": next.Status,
					"stage": next.Stage, "operation_id": next.OperationID, "cpr_key_id": "",
					"key_ciphertext": next.KeyCiphertext, "previous_key_id": next.PreviousKeyID,
					"previous_operation_id": next.PreviousOperationID, "create_issued": false, "stage_attempt": 0, "attempt_until": 0,
					"updated_at": next.UpdatedAt,
				})
			if result.Error != nil {
				return result.Error
			}
			if result.RowsAffected != 1 {
				return ErrCprMemberAccessConflict
			}
		}
		return authorizeCprMemberAccess(tx, identity, next.UserID, next.GroupIDsJSON != "[]", authorize)
	})
}

// CheckCprMemberAccessManagement 复核当前登录会话与目标角色。
func CheckCprMemberAccessManagement(ctx context.Context, identity AuthSessionIdentity, userID int, authorize CprMemberAccessAuthorizer) error {
	return memberAccessDB(ctx).Transaction(func(tx *gorm.DB) error {
		return authorizeCprMemberAccess(tx, identity, userID, false, authorize)
	})
}

// BeginCprMemberRevocation 将原操作转为撤权，保留远端资源的核对材料。
func BeginCprMemberRevocation(ctx context.Context, identity AuthSessionIdentity, access *CprMemberAccess, authorize CprMemberAccessAuthorizer) error {
	if access.Stage != CprMemberDisableOld && access.Stage != CprMemberCreateKey {
		return ErrCprMemberAccessConflict
	}
	now := time.Now().Unix()
	err := memberAccessDB(ctx).Transaction(func(tx *gorm.DB) error {
		result := tx.Model(&CprMemberAccess{}).
			Where("user_id = ? AND revision = ? AND operation_id = ? AND status = ? AND stage = ? AND stage_attempt = ? AND attempt_until = ? AND attempt_until <= ? AND create_issued = ?",
				access.UserID, access.Revision, access.OperationID, access.Status, access.Stage, access.StageAttempt, access.AttemptUntil, now, access.CreateIssued).
			Where("status IN ?", []string{CprMemberAccessPending, CprMemberAccessUnknown}).
			Updates(map[string]any{"status": CprMemberAccessPending, "stage": CprMemberRevokeKeys, "stage_attempt": 0, "attempt_until": 0, "updated_at": now})
		if result.Error != nil {
			return result.Error
		}
		if result.RowsAffected != 1 {
			return ErrCprMemberAccessConflict
		}
		return authorizeCprMemberAccess(tx, identity, access.UserID, false, authorize)
	})
	if err == nil {
		access.Status, access.Stage = CprMemberAccessPending, CprMemberRevokeKeys
		access.StageAttempt, access.AttemptUntil, access.UpdatedAt = 0, 0, now
	}
	return err
}

// MarkCprMemberCreateIssued 先保存当前领取者的发送意图，再创建远端 Key。
func MarkCprMemberCreateIssued(ctx context.Context, access *CprMemberAccess) error {
	if access.Stage != CprMemberCreateKey || access.StageAttempt <= 0 {
		return ErrCprMemberAccessConflict
	}
	now := time.Now().Unix()
	db := memberAccessDB(ctx).Model(&CprMemberAccess{}).
		Where("user_id = ? AND revision = ? AND operation_id = ? AND status = ? AND stage = ? AND stage_attempt = ? AND attempt_until = ? AND attempt_until > ?",
			access.UserID, access.Revision, access.OperationID, CprMemberAccessPending, CprMemberCreateKey, access.StageAttempt, access.AttemptUntil, now)
	if access.CreateIssued {
		// 再次显式创建前核对同一次领取，兼容各数据库的更新计数。
		var count int64
		if err := db.Where("create_issued = ?", true).Count(&count).Error; err != nil {
			return err
		}
		if count != 1 {
			return ErrCprMemberAccessConflict
		}
		return nil
	}
	result := db.Where("create_issued = ?", false).Updates(map[string]any{"create_issued": true, "updated_at": now})
	if result.Error != nil {
		return result.Error
	}
	if result.RowsAffected != 1 {
		return ErrCprMemberAccessConflict
	}
	access.CreateIssued, access.UpdatedAt = true, now
	return nil
}

// ClaimCprMemberAccessStage 按当前操作与阶段领取一次 CPR 调用。
func ClaimCprMemberAccessStage(ctx context.Context, identity AuthSessionIdentity, access *CprMemberAccess, until int64, authorize CprMemberAccessAuthorizer) error {
	if access.StageAttempt == math.MaxInt64 {
		return ErrCprMemberAccessConflict
	}
	now := time.Now().Unix()
	err := memberAccessDB(ctx).Transaction(func(tx *gorm.DB) error {
		result := tx.Model(&CprMemberAccess{}).
			Where("user_id = ? AND revision = ? AND operation_id = ? AND stage = ? AND stage_attempt = ? AND attempt_until <= ? AND create_issued = ? AND status IN ?",
				access.UserID, access.Revision, access.OperationID, access.Stage, access.StageAttempt, now, access.CreateIssued, []string{CprMemberAccessPending, CprMemberAccessUnknown}).
			Updates(map[string]any{"status": CprMemberAccessPending, "stage_attempt": access.StageAttempt + 1, "attempt_until": until, "updated_at": now})
		if result.Error != nil {
			return result.Error
		}
		if result.RowsAffected != 1 {
			return ErrCprMemberAccessConflict
		}
		return authorizeCprMemberAccess(tx, identity, access.UserID, access.Stage != CprMemberRevokeKeys && access.GroupIDsJSON != "[]", authorize)
	})
	if err == nil {
		access.Status = CprMemberAccessPending
		access.StageAttempt++
		access.AttemptUntil = until
		access.UpdatedAt = now
	}
	return err
}

// FinishCprMemberAccessStage 只接收当前领取者的结果，保留同一授权代次。
func FinishCprMemberAccessStage(ctx context.Context, access *CprMemberAccess, nextStatus, nextStage, newKeyID string) error {
	now := time.Now().Unix()
	attempt := access.StageAttempt
	previousKeyID, previousOpID := access.PreviousKeyID, access.PreviousOperationID
	ciphertext, groups, createIssued := access.KeyCiphertext, access.GroupIDsJSON, access.CreateIssued
	if nextStage != access.Stage {
		attempt, previousKeyID, previousOpID = 0, "", ""
	}
	if nextStatus == CprMemberAccessRevoked {
		ciphertext, groups, createIssued = "", "[]", false
	}
	result := memberAccessDB(ctx).Model(&CprMemberAccess{}).
		Where("user_id = ? AND revision = ? AND operation_id = ? AND status = ? AND stage = ? AND stage_attempt = ? AND attempt_until = ? AND create_issued = ?",
			access.UserID, access.Revision, access.OperationID, CprMemberAccessPending, access.Stage, access.StageAttempt, access.AttemptUntil, access.CreateIssued).
		Updates(map[string]any{"status": nextStatus, "stage": nextStage, "cpr_key_id": newKeyID,
			"key_ciphertext": ciphertext, "previous_key_id": previousKeyID, "previous_operation_id": previousOpID,
			"group_ids": groups, "create_issued": createIssued,
			"stage_attempt": attempt, "attempt_until": 0, "updated_at": now})
	if result.Error != nil {
		return result.Error
	}
	if result.RowsAffected != 1 {
		return ErrCprMemberAccessConflict
	}
	access.Status, access.Stage, access.CprKeyID = nextStatus, nextStage, newKeyID
	access.KeyCiphertext, access.PreviousKeyID, access.PreviousOperationID = ciphertext, previousKeyID, previousOpID
	access.GroupIDsJSON, access.CreateIssued = groups, createIssued
	access.StageAttempt, access.AttemptUntil, access.UpdatedAt = attempt, 0, now
	return nil
}
