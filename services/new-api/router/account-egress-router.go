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
	handlePermissionRoute(group, http.MethodGet, "/accounts/detail", authz.AccountEgressRead, controller.GetAccountEgressDetail)
	handlePermissionRoute(group, http.MethodGet, "/account-groups/options", authz.AccountEgressRead, controller.GetAccountEgressGroupOptions)
	handlePermissionRoute(group, http.MethodGet, "/engines", authz.AccountEgressRead, controller.GetAccountEgressEngines)
	handlePermissionRoute(group, http.MethodGet, "/managed-routes", authz.AccountEgressRead, controller.GetAccountEgressManagedRoute)
	handlePermissionRoute(group, http.MethodGet, "/managed-routes/operations", authz.AccountEgressRead, controller.GetAccountEgressRouteOperation)

	accounts := group.Group("", middleware.RequirePermission(authz.AccountEgressManageAccounts), middleware.RequireBrowserSessionOrigin(), middleware.UserCriticalRateLimit("account_egress_accounts"))
	accounts.POST("/accounts/settings", controller.UpdateAccountEgressSettings)
	accounts.POST("/accounts/quota/refresh", controller.RefreshAccountEgressQuota)
	accounts.POST("/members/:id/groups", controller.UpdateCprMemberAccess)
	accounts.POST("/members/:id/reconcile", controller.ReconcileCprMemberAccess)
	group.GET("/members/:id", middleware.RequirePermission(authz.AccountEgressManageAccounts), controller.GetCprMemberAccess)

	egress := group.Group("", middleware.RequirePermission(authz.AccountEgressManageEgress), middleware.RequireBrowserSessionOrigin(), middleware.UserCriticalRateLimit("account_egress_operations"))
	egress.POST("/engines/stop", controller.StopAccountEgressEngine)
	egress.POST("/engines/reclaim", controller.ReclaimAccountEgressEngine)
	egress.POST("/engines/test-node", controller.TestAccountEgressNode)
	egress.POST("/managed-routes/prepare", controller.PrepareAccountEgressManagedRoute)
	egress.POST("/managed-routes/approve", controller.ApproveAccountEgressManagedRoute)
	egress.POST("/managed-routes/abort", controller.AbortAccountEgressManagedRoute)
	egress.POST("/managed-routes/reconcile", controller.ReconcileAccountEgressManagedRoute)

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
