package authz

const ResourceAccountEgress = "account_egress"

var AccountEgressRead = Permission{Resource: ResourceAccountEgress, Action: ActionRead}
var AccountEgressAuthorize = Permission{Resource: ResourceAccountEgress, Action: "authorize"}

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
			Action:         "authorize",
			LabelKey:       "Authorize upstream accounts",
			DescriptionKey: "Start and complete upstream account authorization in your login session.",
			DefaultRoles:   []string{BuiltInRoleAdmin},
		}},
	})
}
