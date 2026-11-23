package controller

import (
	"net/http"
	"strconv"

	"github.com/QuantumNous/new-api/common"
	"github.com/QuantumNous/new-api/service"
	"github.com/gin-gonic/gin"
)

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
