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
import {
  useIsMutating,
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import { useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

import { ConfirmDialog } from '@/components/confirm-dialog'
import { Dialog } from '@/components/dialog'
import { ErrorState } from '@/components/error-state'
import { LoadingState } from '@/components/loading-state'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { toIntlLocale } from '@/i18n/languages'
import {
  ADMIN_PERMISSION_ACTIONS,
  ADMIN_PERMISSION_RESOURCES,
  hasPermission,
} from '@/lib/admin-permissions'
import { formatNumber } from '@/lib/format'
import {
  getServerErrorMessage,
  getServerErrorSources,
  getServerErrorStatus,
} from '@/lib/server-error-message'
import { useAuthStore } from '@/stores/auth-store'

import { AccountGroupEditor } from './account-group-editor'
import {
  getAccountDetail,
  getManagedRoute,
  refreshAccountQuota,
  saveAccountSettings,
  type AccountDetail,
  type AccountGroup,
} from './console-api'
import { ManagedRoutePanel } from './managed-route-panel'
import type { AccountOAuthTarget } from './oauth-types'
import type { Account } from './types'

type AccountDetailsDialogProps = {
  account: Account
  onClose: () => void
  onReauthorize: (target: AccountOAuthTarget) => void
}
type AccountCommand =
  | { kind: 'enabled'; enabled: boolean }
  | { kind: 'groups'; selected: string[]; expected: string[] }
  | { kind: 'quota' }

export function AccountDetailsDialog(props: AccountDetailsDialogProps) {
  const { t } = useTranslation()
  const user = useAuthStore((state) => state.auth.user)
  const sessionId = useAuthStore((state) => state.auth.session?.sid ?? '')
  const userId = user?.id
  const queryClient = useQueryClient()
  const [confirmEnabled, setConfirmEnabled] = useState<boolean | null>(null)
  const [editGroups, setEditGroups] = useState<AccountGroup[] | null>(null)
  const canRead = hasPermission(
    user,
    ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
    ADMIN_PERMISSION_ACTIONS.READ
  )
  const accountId = props.account.id
  const detailKey = [
    'account-egress',
    userId,
    'account-detail',
    sessionId,
    accountId,
  ] as const
  const mutationKey = [
    'account-egress',
    userId,
    'account-write',
    sessionId,
    accountId,
  ] as const
  const detail = useQuery({
    queryKey: detailKey,
    queryFn: ({ signal }) => getAccountDetail(accountId, signal),
    enabled: canRead && !!userId && !!sessionId,
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const route = useQuery({
    queryKey: ['account-egress', userId, 'managed-route', sessionId, accountId],
    queryFn: async ({ signal }) => {
      try {
        return await getManagedRoute(accountId, signal)
      } catch (error) {
        if (getServerErrorStatus(error) === 404) return null
        throw error
      }
    },
    enabled: canRead && !!userId && !!sessionId && detail.isSuccess,
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const change = useMutation({
    mutationKey,
    mutationFn: (command: AccountCommand) => {
      switch (command.kind) {
        case 'enabled':
          return saveAccountSettings({
            account_id: accountId,
            enabled: command.enabled,
          })
        case 'groups':
          return saveAccountSettings({
            account_id: accountId,
            group_ids: command.selected,
            expected_group_ids: command.expected,
          })
        case 'quota':
          return refreshAccountQuota(accountId)
      }
    },
    onSuccess: (latest, command) => {
      queryClient.setQueryData(detailKey, latest)
      if (command.kind === 'groups') setEditGroups(null)
    },
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: ['account-egress', userId] }),
    networkMode: 'always',
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const busy = useIsMutating({ mutationKey, exact: true }) > 0
  const account = detail.isSuccess ? detail.data.account : undefined
  const canManage = hasPermission(
    user,
    ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
    ADMIN_PERMISSION_ACTIONS.MANAGE_ACCOUNTS
  )
  const canManageRoute = hasPermission(
    user,
    ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
    ADMIN_PERMISSION_ACTIONS.MANAGE_EGRESS
  )
  const canAuthorize = hasPermission(
    user,
    ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
    ADMIN_PERMISSION_ACTIONS.AUTHORIZE
  )
  const status = getServerErrorStatus(change.error)
  const conflict = status === 409
  const resultUnknown =
    change.isError &&
    (status == null ||
      getServerErrorSources(change.error).some(
        (source) => source.code === 'account_egress_result_unknown'
      ))
  const needsRefresh =
    conflict ||
    resultUnknown ||
    getServerErrorSources(change.error).some(
      (source) => source.code === 'account_settings_readback_unavailable'
    )
  let message: string | undefined
  if (conflict && change.variables?.kind === 'groups') {
    message = t(
      'Group assignments changed. Reload the account and review its groups.'
    )
  } else if (resultUnknown) {
    message = t(
      'The operation result is unknown. Refresh its status before trying again.'
    )
  } else if (change.isError) {
    message = getServerErrorMessage(change.error)
  }
  const reload = async (): Promise<AccountGroup[] | null> => {
    const latest = await detail.refetch()
    if (latest.isSuccess) {
      change.reset()
      return latest.data.account.groups
    }
    return null
  }
  const actionsDisabled =
    !canManage || busy || detail.isFetching || needsRefresh

  let content: ReactNode
  if (!canRead || !userId || !sessionId) {
    content = (
      <ErrorState title={t('Access to accounts and egress has changed.')} />
    )
  } else if (detail.isError) {
    content = (
      <ErrorState
        title={t('Account details are unavailable')}
        onRetry={() => void reload()}
      />
    )
  } else if (!account) {
    content = <LoadingState />
  } else {
    content = (
      <div className='space-y-4'>
        <section className='space-y-3 rounded-lg border p-3'>
          <div className='flex flex-wrap items-center justify-between gap-2'>
            <h3 className='font-semibold'>{t('Account settings')}</h3>
            {canManage && (
              <Button
                size='sm'
                variant='outline'
                disabled={actionsDisabled}
                onClick={() => {
                  change.reset()
                  setConfirmEnabled(!account.enabled)
                }}
              >
                {account.enabled ? t('Disable account') : t('Enable account')}
              </Button>
            )}
          </div>
          <dl className='grid gap-x-4 gap-y-2 text-sm sm:grid-cols-2'>
            <div>
              <dt>{t('Provider')}</dt>
              <dd>{account.provider}</dd>
            </div>
            <div>
              <dt>{t('Plan')}</dt>
              <dd>{account.planType ?? '—'}</dd>
            </div>
            <div>
              <dt>{t('Status')}</dt>
              <dd>
                <Badge variant='outline'>
                  <AccountStatus
                    status={account.status}
                    enabled={account.enabled}
                  />
                </Badge>
              </dd>
            </div>
            <div>
              <dt>{t('Egress ID')}</dt>
              <dd className='break-all'>{account.outboundProxyId ?? '—'}</dd>
            </div>
          </dl>
          <div className='flex flex-wrap items-center justify-between gap-2'>
            <h4 className='text-sm font-medium'>{t('Groups')}</h4>
            {canManage && (
              <Button
                size='sm'
                variant='outline'
                disabled={actionsDisabled}
                onClick={() => {
                  change.reset()
                  setEditGroups(account.groups)
                }}
              >
                {t('Edit account groups')}
              </Button>
            )}
          </div>
          {account.groups.length ? (
            <ul className='flex flex-wrap gap-1.5' aria-label={t('Groups')}>
              {account.groups.map((group) => (
                <li key={group.id}>
                  <Badge
                    variant='secondary'
                    className='max-w-full break-words whitespace-normal'
                  >
                    {group.name}
                    {group.enabled === false && <> · {t('Disabled')}</>}
                  </Badge>
                </li>
              ))}
            </ul>
          ) : (
            <p className='text-muted-foreground text-sm'>
              {t('No groups assigned')}
            </p>
          )}
          {message && !editGroups && (
            <p role='alert' className='text-destructive text-sm'>
              {message}
            </p>
          )}
          {needsRefresh && !editGroups && (
            <Button
              size='sm'
              variant='outline'
              disabled={busy || detail.isFetching}
              onClick={() => void reload()}
            >
              {t('Reload account')}
            </Button>
          )}
        </section>
        <AccountQuota
          account={account}
          allowRefresh={canManage && account.capabilities.quotaRefresh}
          disabled={actionsDisabled}
          onRefresh={() => {
            if (!actionsDisabled) change.mutate({ kind: 'quota' })
          }}
        />
        <ManagedRoutePanel
          account={account}
          userId={userId}
          sessionId={sessionId}
          route={route}
          mutationKey={mutationKey}
          canManage={canManageRoute}
          canAuthorize={canAuthorize}
          onReauthorize={props.onReauthorize}
        />
        {editGroups && canManage && (
          <AccountGroupEditor
            userId={userId}
            sessionId={sessionId}
            initialGroups={editGroups}
            pending={busy}
            needsRefresh={needsRefresh}
            message={message}
            onClose={() => {
              change.reset()
              setEditGroups(null)
            }}
            onSave={(selected, expected) => {
              if (!actionsDisabled) {
                change.mutate({ kind: 'groups', selected, expected })
              }
            }}
            onReload={reload}
          />
        )}
        {confirmEnabled !== null && canManage && (
          <ConfirmDialog
            open
            onOpenChange={(open) => {
              if (!open && !busy) setConfirmEnabled(null)
            }}
            title={confirmEnabled ? t('Enable account') : t('Disable account')}
            desc={
              confirmEnabled
                ? t(
                    'Allow new requests to use this account when its route is ready.'
                  )
                : t('Pause new requests to this upstream account.')
            }
            confirmText={
              confirmEnabled ? t('Enable account') : t('Disable account')
            }
            disabled={actionsDisabled}
            destructive={!confirmEnabled}
            isLoading={busy}
            handleConfirm={() => {
              if (actionsDisabled) return
              const enabled = confirmEnabled
              setConfirmEnabled(null)
              change.mutate({ kind: 'enabled', enabled })
            }}
          />
        )}
      </div>
    )
  }
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) props.onClose()
      }}
      title={t('Account details')}
      description={<span className='break-all'>{props.account.name}</span>}
      footer={
        <Button variant='outline' disabled={busy} onClick={props.onClose}>
          {t('Close')}
        </Button>
      }
      contentClassName='sm:max-w-3xl'
    >
      {content}
    </Dialog>
  )
}

function AccountStatus(props: { status: string; enabled: boolean }) {
  const { t } = useTranslation()
  if (!props.enabled) return t('Disabled')
  switch (props.status) {
    case 'normal':
      return t('Normal')
    case 'quota_paused':
      return t('Quota paused')
    case 'quota_exhausted':
      return t('Quota exhausted')
    case 'rate_limited':
      return t('Rate limited')
    case 'disabled':
      return t('Disabled')
    case 'error':
      return t('Error')
    default:
      return t('Unknown')
  }
}

function AccountQuota(props: {
  account: AccountDetail
  allowRefresh: boolean
  disabled: boolean
  onRefresh: () => void
}) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const { quota, capacity } = props.account
  const windows = quota?.windows ?? []
  if (
    !props.account.capabilities.quota &&
    !props.allowRefresh &&
    capacity?.usedSlots == null
  ) {
    return null
  }

  return (
    <section className='space-y-3 rounded-lg border p-3'>
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <h3 className='font-semibold'>{t('Account quota')}</h3>
        {props.allowRefresh && (
          <Button
            size='sm'
            variant='outline'
            disabled={props.disabled}
            onClick={props.onRefresh}
          >
            {t('Refresh quota')}
          </Button>
        )}
      </div>
      {capacity?.usedSlots != null && (
        <p className='text-sm'>
          {t('Account slots in use')}:{' '}
          {formatNumber(capacity.usedSlots, locale)}
          {capacity.totalSlots != null && (
            <> / {formatNumber(capacity.totalSlots, locale)}</>
          )}
        </p>
      )}
      {quota?.refreshedAt && (
        <p className='text-muted-foreground text-sm'>
          {t('Last refreshed')}:{' '}
          <time dateTime={quota.refreshedAt}>
            {new Date(quota.refreshedAt).toLocaleString(locale)}
          </time>
        </p>
      )}
      {quota?.rateLimitedUntil && (
        <p className='text-sm'>
          {t('Rate limited until')}:{' '}
          <time dateTime={quota.rateLimitedUntil}>
            {new Date(quota.rateLimitedUntil).toLocaleString(locale)}
          </time>
        </p>
      )}
      {quota?.limitReached && (
        <Badge variant='destructive'>{t('Quota exhausted')}</Badge>
      )}
      {quota?.credits?.hasCredits && (
        <p className='text-sm'>
          {t('Credits')}:{' '}
          {quota.credits.unlimited
            ? t('Unlimited')
            : (quota.credits.balance ?? '—')}
        </p>
      )}
      {windows.length > 0 ? (
        <ul className='grid gap-2 text-sm sm:grid-cols-2'>
          {windows.map((window) => (
            <li
              key={window.key}
              className='min-w-0 space-y-1 rounded-md border p-2'
            >
              <p className='font-medium break-words'>
                {window.labelDisplay || t('Quota window')}
              </p>
              <div className='flex flex-wrap items-center gap-1.5'>
                <span>
                  {window.usedPercent == null
                    ? t('Usage unavailable')
                    : t('{{value}}% used', {
                        value: formatNumber(window.usedPercent, locale),
                      })}
                </span>
                {window.stale && <Badge variant='outline'>{t('Stale')}</Badge>}
                {window.limitReached && (
                  <Badge variant='destructive'>{t('Quota exhausted')}</Badge>
                )}
                {quota?.pausedWindows?.includes(window.key) && (
                  <Badge variant='outline'>{t('Quota paused')}</Badge>
                )}
              </div>
              {window.resetAtDisplay && (
                <p className='text-muted-foreground break-words'>
                  {t('Resets at')}: {window.resetAtDisplay}
                </p>
              )}
            </li>
          ))}
        </ul>
      ) : (
        props.account.capabilities.quota && (
          <p className='text-muted-foreground text-sm'>
            {t('Quota observations are unavailable.')}
          </p>
        )
      )}
    </section>
  )
}
