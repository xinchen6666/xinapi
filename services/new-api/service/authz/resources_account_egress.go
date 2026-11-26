package authz

const (
	ResourceAccountEgress = "account_egress"
	actionAuthorize       = "authorize"
	actionManageSources   = "manage_sources"
	actionManageEgress    = "manage_egress"
	actionManageAccounts  = "manage_accounts"
)

var (
	AccountEgressRead           = Permission{Resource: ResourceAccountEgress, Action: ActionRead}
	AccountEgressAuthorize      = Permission{Resource: ResourceAccountEgress, Action: actionAuthorize}
	AccountEgressManageSources  = Permission{Resource: ResourceAccountEgress, Action: actionManageSources}
	AccountEgressManageEgress   = Permission{Resource: ResourceAccountEgress, Action: actionManageEgress}
	AccountEgressManageAccounts = Permission{Resource: ResourceAccountEgress, Action: actionManageAccounts}
)

func init() {
	RegisterResource(ResourceDefinition{
		Resource: ResourceAccountEgress,
		LabelKey: "Accounts and Egress",
		Actions: []ActionDefinition{{
			Action:         ActionRead,
			LabelKey:       "Read accounts and egress",
			DescriptionKey: "View account and egress lists without credentials.",
			DefaultRoles:   []string{BuiltInRoleAdmin},
		}, {
			Action:         actionAuthorize,
			LabelKey:       "Authorize upstream accounts",
			DescriptionKey: "Start and complete upstream account authorization in your login session.",
			DefaultRoles:   []string{BuiltInRoleAdmin},
		}, {
			Action:         actionManageSources,
			LabelKey:       "Manage subscription sources",
			DescriptionKey: "Preview subscription links, manage sources, and approve versions.",
		}, {
			Action:         actionManageEgress,
			LabelKey:       "Manage account egress",
			DescriptionKey: "Manage account routes and their egress engines.",
		}, {
			Action:         actionManageAccounts,
			LabelKey:       "Manage upstream accounts",
			DescriptionKey: "Manage upstream account settings and group assignments.",
		}},
	})
}
