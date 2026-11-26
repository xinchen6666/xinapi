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
	handlePermissionRoute(group, http.MethodGet, "/subscription-sources", authz.AccountEgressRead, controller.GetSubscriptionSources)
	handlePermissionRoute(group, http.MethodGet, "/subscription-sources/detail", authz.AccountEgressRead, controller.GetSubscriptionSourceDetail)

	sources := group.Group("", middleware.RequirePermission(authz.AccountEgressManageSources), middleware.RequireBrowserSessionOrigin(), middleware.UserCriticalRateLimit("account_egress_sources"))
	sources.POST("/subscription-preview", controller.PreviewAccountEgressSubscription)
	sources.POST("/subscription-sources/create", controller.CreateSubscriptionSource)
	sources.POST("/subscription-sources/update", controller.UpdateSubscriptionSource)
	sources.POST("/subscription-sources/refresh", controller.RefreshSubscriptionSource)
	sources.POST("/subscription-sources/approve", controller.ApproveSubscriptionSource)

	oauth := group.Group("/oauth/operations", middleware.RequirePermission(authz.AccountEgressAuthorize))
	oauth.GET("", controller.ListCprOAuthOperations)
	oauth.GET("/:operation_id", controller.GetCprOAuthOperation)
	writes := oauth.Group("", middleware.RequireBrowserSessionOrigin(), middleware.CriticalRateLimit())
	writes.POST("", controller.StartCprOAuthOperation)
	writes.POST("/:operation_id/complete", controller.CompleteCprOAuthOperation)
	writes.POST("/:operation_id/cancel", controller.CancelCprOAuthOperation)
}
