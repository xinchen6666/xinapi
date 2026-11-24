package router

import (
	"net/http"

	"github.com/QuantumNous/new-api/controller"
	"github.com/QuantumNous/new-api/middleware"
	"github.com/QuantumNous/new-api/service/authz"
	"github.com/gin-gonic/gin"
)

func registerAccountEgressRoutes(apiRouter *gin.RouterGroup) {
	group := apiRouter.Group("/account-egress", middleware.AdminAuth(), middleware.DisableCache())
	handlePermissionRoute(group, http.MethodGet, "/accounts", authz.AccountEgressRead, controller.GetAccountEgressAccounts)
	handlePermissionRoute(group, http.MethodGet, "/proxies", authz.AccountEgressRead, controller.GetAccountEgressProxies)
	handlePermissionRoute(group, http.MethodPost, "/subscription-preview", authz.AccountEgressRead, controller.PreviewAccountEgressSubscription)
}
