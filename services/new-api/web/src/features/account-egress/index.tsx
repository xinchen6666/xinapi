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
import { useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ErrorState } from '@/components/error-state'
import { SectionPageLayout } from '@/components/layout'
import { Button } from '@/components/ui/button'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { toIntlLocale } from '@/i18n/languages'
import {
  ADMIN_PERMISSION_RESOURCES,
  ADMIN_PERMISSION_ACTIONS,
  hasPermission,
} from '@/lib/admin-permissions'
import { formatNumber } from '@/lib/format'
import { ROLE } from '@/lib/roles'
import { useAuthStore } from '@/stores/auth-store'

import { getAccounts, getEgressProxies } from './api'
import { OAuthPanel } from './oauth-panel'
import type { AccountOAuthTarget } from './oauth-types'
import { ReadOnlyList } from './read-only-list'
import type { Account, EgressProxy } from './types'

const STATUS_LABELS: Record<string, string> = {
  normal: 'Normal',
  quota_paused: 'Quota paused',
  quota_exhausted: 'Quota exhausted',
  rate_limited: 'Rate limited',
  disabled: 'Disabled',
  error: 'Error',
}

export function AccountEgress() {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const user = useAuthStore((state) => state.auth.user)
  const queryClient = useQueryClient()
  const [kind, setKind] = useState<'accounts' | 'proxies' | 'oauth'>('accounts')
  const [startTarget, setStartTarget] = useState<
    AccountOAuthTarget | 'new' | null
  >(null)
  const sessionId = useAuthStore((state) => state.auth.session?.sid ?? '')
  const canAuthorize =
    !!user &&
    user.role >= ROLE.ADMIN &&
    hasPermission(
      user,
      ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
      ADMIN_PERMISSION_ACTIONS.AUTHORIZE
    )
  const [accessRevoked, setAccessRevoked] = useState(false)
  const userId = user?.id
  useEffect(() => {
    setStartTarget(null)
    setKind('accounts')
    setAccessRevoked(false)
  }, [userId, sessionId])
  const canRead =
    !!user &&
    user.role >= ROLE.ADMIN &&
    hasPermission(
      user,
      ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
      ADMIN_PERMISSION_ACTIONS.READ
    )
  const handleAuthorizationDenied = useCallback(() => {
    setStartTarget(null)
    setKind('accounts')
    const current = useAuthStore.getState().auth.user
    if (current && current.id === userId) {
      useAuthStore.getState().auth.setUser({
        ...current,
        permissions: {
          ...current.permissions,
          admin_permissions: {
            ...current.permissions?.admin_permissions,
            account_egress: {
              ...current.permissions?.admin_permissions?.account_egress,
              authorize: false,
            },
          },
        },
      })
    }
    void queryClient
      .cancelQueries({ queryKey: ['account-egress', userId, 'oauth'] })
      .then(() => {
        queryClient.removeQueries({
          queryKey: ['account-egress', userId, 'oauth'],
        })
      })
  }, [queryClient, userId])
  const handleAccessDenied = useCallback(() => {
    setAccessRevoked(true)
    const current = useAuthStore.getState().auth.user
    if (current && current.id === userId) {
      useAuthStore.getState().auth.setUser({
        ...current,
        permissions: {
          ...current.permissions,
          admin_permissions: {
            ...current.permissions?.admin_permissions,
            account_egress: { read: false },
          },
        },
      })
    }
    void queryClient
      .cancelQueries({ queryKey: ['account-egress', userId] })
      .then(() => {
        queryClient.removeQueries({ queryKey: ['account-egress', userId] })
      })
  }, [queryClient, userId])
  useEffect(() => {
    if (!canRead) {
      void queryClient
        .cancelQueries({ queryKey: ['account-egress', userId] })
        .then(() => {
          queryClient.removeQueries({ queryKey: ['account-egress', userId] })
        })
    }
  }, [canRead, queryClient, userId])
  const accountColumns: ColumnDef<Account>[] = [
    {
      accessorKey: 'name',
      header: t('Name'),
      size: 200,
      meta: { mobileTitle: true },
    },
    { accessorKey: 'provider', header: t('Provider') },
    {
      accessorKey: 'status',
      header: t('Status'),
      cell: ({ row }) =>
        t(STATUS_LABELS[row.original.status] ?? row.original.status, {
          defaultValue: row.original.status,
        }),
    },
    {
      accessorKey: 'enabled',
      header: t('Enabled'),
      cell: ({ row }) => (row.original.enabled ? t('Yes') : t('No')),
    },
    {
      accessorKey: 'planType',
      header: t('Plan'),
      cell: ({ row }) => row.original.planType ?? '—',
    },
    {
      accessorKey: 'outboundProxyId',
      header: t('Egress ID'),
      cell: ({ row }) => (
        <span className='break-all'>{row.original.outboundProxyId ?? '—'}</span>
      ),
    },
    {
      accessorKey: 'id',
      header: t('ID'),
      size: 280,
      cell: ({ row }) => <span className='break-all'>{row.original.id}</span>,
    },
  ]
  if (canAuthorize) {
    accountColumns.push({
      id: 'authorize',
      header: t('Actions'),
      cell: ({ row }) => {
        const account = row.original
        if (account.provider !== 'openai' && account.provider !== 'xai') {
          return '—'
        }
        return (
          <Button
            variant='outline'
            size='sm'
            onClick={() => {
              setStartTarget({
                account_id: account.id,
                provider: account.provider as AccountOAuthTarget['provider'],
                name: account.name,
              })
              setKind('oauth')
            }}
          >
            {t('Reauthorize')}
          </Button>
        )
      },
    })
  }
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
  return (
    <SectionPageLayout fixedContent stackActionsOnMobile>
      <SectionPageLayout.Title>
        {t('Accounts and Egress')}
      </SectionPageLayout.Title>
      {canRead && !accessRevoked && (
        <SectionPageLayout.Actions>
          <Tabs
            value={kind}
            onValueChange={(value) =>
              setKind(
                value === 'proxies' || value === 'oauth' ? value : 'accounts'
              )
            }
          >
            <TabsList aria-label={t('Accounts and Egress')}>
              <TabsTrigger value='accounts'>
                {t('Upstream accounts')}
              </TabsTrigger>
              <TabsTrigger value='proxies'>{t('Egress proxies')}</TabsTrigger>
              {canAuthorize && (
                <TabsTrigger value='oauth'>
                  {t('Account authorizations')}
                </TabsTrigger>
              )}
            </TabsList>
          </Tabs>
        </SectionPageLayout.Actions>
      )}
      <SectionPageLayout.Content>
        {canRead && !accessRevoked ? (
          <div className='flex h-full min-h-0 flex-col gap-3'>
            <p className='text-muted-foreground text-sm'>
              {kind === 'oauth'
                ? t(
                    'Manage upstream authorizations and restore their session receipts.'
                  )
                : t('View account status and egress assignments.')}
            </p>
            {kind === 'oauth' && canAuthorize && (
              <OAuthPanel
                key={`${userId}:${sessionId}:oauth`}
                userId={user.id}
                sessionId={sessionId}
                startTarget={startTarget}
                onStart={() => setStartTarget('new')}
                onCloseStart={() => setStartTarget(null)}
                onAccessDenied={handleAuthorizationDenied}
              />
            )}
            {kind === 'accounts' && (
              <ReadOnlyList
                key={`${userId}:accounts`}
                kind='accounts'
                userId={user.id}
                columns={accountColumns}
                load={getAccounts}
                onAccessDenied={handleAccessDenied}
              />
            )}
            {kind === 'proxies' && (
              <ReadOnlyList
                key={`${userId}:proxies`}
                kind='proxies'
                userId={user.id}
                columns={proxyColumns}
                load={getEgressProxies}
                onAccessDenied={handleAccessDenied}
              />
            )}
          </div>
        ) : (
          <ErrorState title={t('Access to accounts and egress has changed.')} />
        )}
      </SectionPageLayout.Content>
    </SectionPageLayout>
  )
}
