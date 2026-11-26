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
	oauth := group.Group("/oauth/operations", middleware.RequirePermission(authz.AccountEgressAuthorize))
	oauth.GET("", controller.ListCprOAuthOperations)
	oauth.GET("/:operation_id", controller.GetCprOAuthOperation)
	oauth.POST("", middleware.SessionCookieOriginGuard(), middleware.CriticalRateLimit(), controller.StartCprOAuthOperation)
	oauth.POST("/:operation_id/complete", middleware.SessionCookieOriginGuard(), middleware.CriticalRateLimit(), controller.CompleteCprOAuthOperation)
	oauth.POST("/:operation_id/cancel", middleware.SessionCookieOriginGuard(), middleware.CriticalRateLimit(), controller.CancelCprOAuthOperation)
	handlePermissionRoute(group, http.MethodGet, "/accounts", authz.AccountEgressRead, controller.GetAccountEgressAccounts)
	handlePermissionRoute(group, http.MethodGet, "/proxies", authz.AccountEgressRead, controller.GetAccountEgressProxies)
	handlePermissionRoute(group, http.MethodPost, "/subscription-preview", authz.AccountEgressRead, controller.PreviewAccountEgressSubscription)
	handlePermissionRoute(group, http.MethodGet, "/subscription-sources", authz.AccountEgressRead, controller.GetSubscriptionSources)
	handlePermissionRoute(group, http.MethodGet, "/subscription-sources/detail", authz.AccountEgressRead, controller.GetSubscriptionSourceDetail)
	group.POST("/subscription-sources/create", middleware.RootAuth(), middleware.RequirePermission(authz.AccountEgressRead), controller.CreateSubscriptionSource)
	group.POST("/subscription-sources/update", middleware.RootAuth(), middleware.RequirePermission(authz.AccountEgressRead), controller.UpdateSubscriptionSource)
	group.POST("/subscription-sources/refresh", middleware.RootAuth(), middleware.RequirePermission(authz.AccountEgressRead), controller.RefreshSubscriptionSource)
	group.POST("/subscription-sources/approve", middleware.RootAuth(), middleware.RequirePermission(authz.AccountEgressRead), controller.ApproveSubscriptionSource)
}
