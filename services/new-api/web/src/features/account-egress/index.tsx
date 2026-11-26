/*
Copyright (C) 2023-2026 QuantumNous

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.

For commercial licensing, please contact support@quantumnous.com
*/
import { useQueryClient } from '@tanstack/react-query'
import type { ColumnDef } from '@tanstack/react-table'
import { useCallback, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ErrorState } from '@/components/error-state'
import { SectionPageLayout } from '@/components/layout'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { SubscriptionPreview } from '@/features/subscription-preview'
import { toIntlLocale } from '@/i18n/languages'
import {
  ADMIN_PERMISSION_RESOURCES,
  ADMIN_PERMISSION_ACTIONS,
  hasPermission,
} from '@/lib/admin-permissions'
import { formatNumber } from '@/lib/format'
import { ROLE } from '@/lib/roles'
import { useAuthStore } from '@/stores/auth-store'

import { AccountTable } from './account-table'
import { getEgressProxies } from './api'
import { EnginePanel } from './engine-panel'
import { OAuthPanel } from './oauth-panel'
import type { AccountOAuthTarget } from './oauth-types'
import { ReadOnlyList } from './read-only-list'
import type { EgressProxy } from './types'
import { egressSectionSchema, type EgressSection } from './workspace-section'

type WorkspaceProps = {
  section: EgressSection
  onSectionChange: (next: EgressSection) => void
}

export function AccountEgress(props: WorkspaceProps) {
  const userId = useAuthStore((state) => state.auth.user?.id)
  const sessionId = useAuthStore((state) => state.auth.session?.sid ?? '')
  return (
    <Workspace
      key={`${userId}:${sessionId}`}
      {...props}
      sessionId={sessionId}
    />
  )
}

function Workspace(props: WorkspaceProps & { sessionId: string }) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const user = useAuthStore((state) => state.auth.user)
  const queryClient = useQueryClient()
  const onSectionChange = props.onSectionChange
  const [startTarget, setStartTarget] = useState<
    AccountOAuthTarget | 'new' | null
  >(null)
  const [accessRevoked, setAccessRevoked] = useState(false)
  const userId = user?.id
  const canRead =
    !!user &&
    user.role >= ROLE.ADMIN &&
    hasPermission(
      user,
      ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
      ADMIN_PERMISSION_ACTIONS.READ
    )
  const canAuthorize =
    canRead &&
    hasPermission(
      user,
      ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
      ADMIN_PERMISSION_ACTIONS.AUTHORIZE
    )
  const section =
    props.section === 'oauth' && !canAuthorize ? 'accounts' : props.section

  const clearPermission = useCallback(
    (action: 'read' | 'authorize') => {
      const auth = useAuthStore.getState().auth
      const current = auth.user
      if (
        current &&
        current.id === userId &&
        (auth.session?.sid ?? '') === props.sessionId
      ) {
        auth.setUser({
          ...current,
          permissions: {
            ...current.permissions,
            admin_permissions: {
              ...current.permissions?.admin_permissions,
              account_egress: {
                ...current.permissions?.admin_permissions?.account_egress,
                [action]: false,
              },
            },
          },
        })
      }
      const scope =
        action === 'authorize'
          ? ['account-egress', userId, 'oauth']
          : ['account-egress', userId]
      void queryClient
        .cancelQueries({ queryKey: scope })
        .then(() => queryClient.removeQueries({ queryKey: scope }))
    },
    [props.sessionId, queryClient, userId]
  )

  const handleAuthorizationDenied = useCallback(() => {
    setStartTarget(null)
    onSectionChange('accounts')
    clearPermission('authorize')
  }, [onSectionChange, clearPermission])
  const handleAccessDenied = useCallback(() => {
    setAccessRevoked(true)
    clearPermission('read')
  }, [clearPermission])

  const proxyColumns: ColumnDef<EgressProxy>[] = [
    {
      accessorKey: 'name',
      header: t('Name'),
      size: 200,
      meta: { mobileTitle: true },
    },
    {
      accessorKey: 'enabled',
      header: t('Enabled'),
      cell: ({ row }) => (row.original.enabled ? t('Yes') : t('No')),
    },
    {
      accessorKey: 'accountCount',
      header: t('Linked accounts'),
      cell: ({ row }) => formatNumber(row.original.accountCount, locale),
    },
    {
      accessorKey: 'hasAuthentication',
      header: t('Authentication configured'),
      cell: ({ row }) => (row.original.hasAuthentication ? t('Yes') : t('No')),
    },
    {
      accessorKey: 'id',
      header: t('ID'),
      size: 280,
      cell: ({ row }) => <span className='break-all'>{row.original.id}</span>,
    },
  ]
  const tableSection =
    section === 'accounts' || section === 'proxies' || section === 'oauth'
  return (
    <Tabs
      className='min-h-0 flex-1'
      value={section}
      onValueChange={(value) => {
        const choice = egressSectionSchema.safeParse(value)
        if (choice.success) onSectionChange(choice.data)
      }}
    >
      <SectionPageLayout fixedContent={tableSection} stackActionsOnMobile>
        <SectionPageLayout.Title>
          {t('Accounts and Egress')}
        </SectionPageLayout.Title>
        {canRead && !accessRevoked && (
          <SectionPageLayout.Actions>
            <TabsList
              aria-label={t('Accounts and Egress')}
              className='h-auto! max-w-full flex-wrap justify-start'
            >
              <TabsTrigger value='accounts'>
                {t('Upstream accounts')}
              </TabsTrigger>
              <TabsTrigger value='sources'>
                {t('Sources and nodes')}
              </TabsTrigger>
              {canAuthorize && (
                <TabsTrigger value='oauth'>
                  {t('Account authorizations')}
                </TabsTrigger>
              )}
              <TabsTrigger value='engines'>{t('Engines')}</TabsTrigger>
              <TabsTrigger value='proxies'>{t('Egress proxies')}</TabsTrigger>
            </TabsList>
          </SectionPageLayout.Actions>
        )}
        <SectionPageLayout.Content>
          {canRead && !accessRevoked ? (
            <>
              <TabsContent
                value='accounts'
                className='flex h-full min-h-0 flex-col'
              >
                <AccountTable
                  userId={user.id}
                  sessionId={props.sessionId}
                  onAccessDenied={handleAccessDenied}
                  onReauthorize={(target) => {
                    setStartTarget(target)
                    onSectionChange('oauth')
                  }}
                />
              </TabsContent>
              <TabsContent value='sources'>
                <SubscriptionPreview onReadDenied={handleAccessDenied} />
              </TabsContent>
              <TabsContent value='engines'>
                <EnginePanel />
              </TabsContent>
              {canAuthorize && (
                <TabsContent
                  value='oauth'
                  className='flex h-full min-h-0 flex-col'
                >
                  <OAuthPanel
                    userId={user.id}
                    sessionId={props.sessionId}
                    startTarget={startTarget}
                    onStart={() => setStartTarget('new')}
                    onCloseStart={() => setStartTarget(null)}
                    onAccessDenied={handleAuthorizationDenied}
                  />
                </TabsContent>
              )}
              <TabsContent
                value='proxies'
                className='flex h-full min-h-0 flex-col'
              >
                <ReadOnlyList
                  kind='proxies'
                  userId={user.id}
                  sessionId={props.sessionId}
                  columns={proxyColumns}
                  load={getEgressProxies}
                  onAccessDenied={handleAccessDenied}
                />
              </TabsContent>
            </>
          ) : (
            <ErrorState
              title={t('Access to accounts and egress has changed.')}
            />
          )}
        </SectionPageLayout.Content>
      </SectionPageLayout>
    </Tabs>
  )
}
