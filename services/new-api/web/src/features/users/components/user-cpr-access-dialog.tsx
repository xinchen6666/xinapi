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
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ConfirmDialog } from '@/components/confirm-dialog'
import { Dialog } from '@/components/dialog'
import { ErrorState } from '@/components/error-state'
import { LoadingState } from '@/components/loading-state'
import { MultiSelect, type Option } from '@/components/multi-select'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Field, FieldLabel } from '@/components/ui/field'
import {
  Pagination,
  PaginationContent,
  PaginationItem,
} from '@/components/ui/pagination'
import { getAccountGroups } from '@/features/account-egress/console-api'
import {
  changeMemberAccess,
  getMemberAccess,
  type MemberAccess,
  type MemberAccessCommand,
} from '@/features/account-egress/member-api'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber } from '@/lib/format'
import {
  getServerErrorStatus,
  isServerErrorCancelled,
} from '@/lib/server-error-message'

import { USER_STATUS } from '../constants'
import type { User } from '../types'

type UserCprAccessDialogProps = {
  user: Pick<User, 'id' | 'username' | 'group' | 'status'>
  operatorId: number
  sessionId: string
  onClose: () => void
}

export function UserCprAccessDialog(props: UserCprAccessDialogProps) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const userId = props.user.id
  const accessKey = [
    'account-egress',
    props.operatorId,
    'member-access',
    props.sessionId,
    userId,
  ] as const
  const [selection, setSelection] = useState<string[] | null>(null)
  const [command, setCommand] = useState<MemberAccessCommand | null>(null)
  const submitting = useRef(false)
  const access = useQuery({
    queryKey: accessKey,
    queryFn: ({ signal }) => getMemberAccess(userId, props.sessionId, signal),
    gcTime: 0,
    retry: false,
    refetchOnMount: 'always',
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    meta: { errorToast: false },
  })
  const change = useMutation({
    mutationFn: (next: MemberAccessCommand) =>
      changeMemberAccess(userId, props.sessionId, next),
    gcTime: 0,
    networkMode: 'always',
    retry: false,
    meta: { errorToast: false },
  })
  const current = access.data
  const selected = selection ?? current?.group_ids ?? []
  const targetEnabled = props.user.status === USER_STATUS.ENABLED
  const unfinished =
    current?.status === 'pending' || current?.status === 'unknown'
  const canAct =
    access.isSuccess &&
    !access.isFetching &&
    !change.isPending &&
    !change.isError
  const canEdit = canAct && !unfinished
  const changed =
    current != null &&
    (selected.length !== current.group_ids.length ||
      selected.some((id) => !current.group_ids.includes(id)))
  const reload = async () => {
    const latest = await access.refetch()
    if (latest.isSuccess) {
      setSelection(null)
      change.reset()
    }
  }
  const submit = () => {
    if (!command || !current || !canAct || submitting.current) return
    if (command.expected_revision !== current.revision) return
    if (command.action === 'reconcile') {
      if (!unfinished || command.operation_id !== current.operation_id) return
    } else if (unfinished || (!targetEnabled && command.group_ids.length > 0)) {
      return
    }
    submitting.current = true
    change.mutate(command, {
      onSuccess: (result) => {
        queryClient.setQueryData(accessKey, result)
        setSelection(null)
      },
      onSettled: () => {
        submitting.current = false
        setCommand(null)
      },
    })
  }
  const conflict = getServerErrorStatus(change.error) === 409
  const showWriteError = change.isError && !isServerErrorCancelled(change.error)
  let confirmTitle = t('Grant upstream access')
  let confirmDescription = t(
    'Grant CPR model access to the selected account groups.'
  )
  const revoking =
    command?.action === 'groups' && command.group_ids.length === 0
  if (revoking) {
    confirmTitle = t('Revoke upstream access')
    confirmDescription = t(
      'Revoke CPR model access by clearing all account groups.'
    )
  } else if (command?.action === 'reconcile') {
    confirmTitle = t('Reconcile access')
    confirmDescription = t(
      'Check and continue the existing upstream access operation.'
    )
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !change.isPending) props.onClose()
      }}
      title={t('Upstream access')}
      description={t('Manage CPR model access for {{username}}.', {
        username: props.user.username,
      })}
      footer={
        <>
          <Button
            variant='outline'
            disabled={change.isPending}
            onClick={props.onClose}
          >
            {t('Close')}
          </Button>
          {current && unfinished && (
            <Button
              disabled={!canAct || !current.operation_id}
              onClick={() => {
                if (current.operation_id) {
                  setCommand({
                    action: 'reconcile',
                    expected_revision: current.revision,
                    operation_id: current.operation_id,
                  })
                }
              }}
            >
              {t('Reconcile access')}
            </Button>
          )}
          {current && !unfinished && current.group_ids.length > 0 && (
            <Button
              variant='outline'
              disabled={!canEdit}
              onClick={() =>
                setCommand({
                  action: 'groups',
                  expected_revision: current.revision,
                  group_ids: [],
                })
              }
            >
              {t('Revoke upstream access')}
            </Button>
          )}
          {current && !unfinished && targetEnabled && (
            <Button
              disabled={!canEdit || !changed}
              onClick={() =>
                setCommand({
                  action: 'groups',
                  expected_revision: current.revision,
                  group_ids: [...selected],
                })
              }
            >
              {t('Save changes')}
            </Button>
          )}
        </>
      }
    >
      {access.isError && (
        <ErrorState
          title={t('Upstream access is unavailable.')}
          onRetry={() => void reload()}
        />
      )}
      {access.isPending && <LoadingState />}
      {access.isSuccess && current && (
        <div className='space-y-4'>
          <MemberAccessStatus
            access={current}
            platformGroup={props.user.group}
          />
          <p className='text-muted-foreground text-sm'>
            {t(
              'Platform groups and CPR account groups are separate. Members without an upstream grant cannot send CPR model requests.'
            )}
          </p>
          {unfinished && (
            <p role='status' className='text-muted-foreground text-sm'>
              {t(
                'An upstream access operation is pending. Reconcile it before making changes.'
              )}
            </p>
          )}
          {!targetEnabled && (
            <p role='status' className='text-muted-foreground text-sm'>
              {t('Disabled members can have their upstream access revoked.')}
            </p>
          )}
          <MemberGroupPicker
            operatorId={props.operatorId}
            sessionId={props.sessionId}
            userId={userId}
            selected={selected}
            disabled={!canEdit || !targetEnabled}
            onChange={setSelection}
          />
          {showWriteError && (
            <p role='alert' className='text-destructive text-sm'>
              {conflict
                ? t(
                    'Upstream access has changed. Reload and review it before continuing.'
                  )
                : t(
                    'The operation result is unknown. Refresh its status before trying again.'
                  )}
            </p>
          )}
          <Button
            variant='outline'
            size='sm'
            disabled={access.isFetching || change.isPending}
            onClick={() => void reload()}
          >
            {t('Reload upstream access')}
          </Button>
        </div>
      )}
      {command && (
        <ConfirmDialog
          open
          onOpenChange={(open) => {
            if (!open && !change.isPending) setCommand(null)
          }}
          title={confirmTitle}
          desc={confirmDescription}
          confirmText={confirmTitle}
          destructive={revoking}
          disabled={!canAct}
          isLoading={change.isPending}
          handleConfirm={submit}
        />
      )}
    </Dialog>
  )
}

function MemberAccessStatus(props: {
  access: MemberAccess
  platformGroup: string
}) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  let status = t('Unknown')
  switch (props.access.status) {
    case 'active':
      status = t('Active')
      break
    case 'pending':
      status = t('Pending')
      break
    case 'revoked':
      status = props.access.revision === 0 ? t('No access') : t('Revoked')
      break
  }
  return (
    <dl className='grid grid-cols-2 gap-3 text-sm sm:grid-cols-3'>
      <div>
        <dt className='text-muted-foreground'>{t('Status')}</dt>
        <dd role='status'>
          <Badge variant='outline'>{status}</Badge>
        </dd>
      </div>
      <div>
        <dt className='text-muted-foreground'>{t('Revision')}</dt>
        <dd>{formatNumber(props.access.revision, locale)}</dd>
      </div>
      <div className='min-w-0'>
        <dt className='text-muted-foreground'>{t('Platform group')}</dt>
        <dd className='break-words'>{props.platformGroup}</dd>
      </div>
    </dl>
  )
}

type MemberGroupPickerProps = {
  operatorId: number
  sessionId: string
  userId: number
  selected: string[]
  disabled: boolean
  onChange: (ids: string[]) => void
}

function MemberGroupPicker(props: MemberGroupPickerProps) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const fieldId = useId()
  const [page, setPage] = useState(1)
  const [selectedNames, setSelectedNames] = useState<Record<string, string>>({})
  const groups = useQuery({
    queryKey: [
      'account-egress',
      props.operatorId,
      'member-account-groups',
      props.sessionId,
      props.userId,
      page,
    ],
    queryFn: ({ signal }) => getAccountGroups(page, signal),
    gcTime: 0,
    retry: false,
    meta: { errorToast: false },
  })
  const items = groups.data?.items ?? []
  const selectedSet = new Set(props.selected)
  const options = new Map<string, Option>(
    props.selected.map((id) => [
      id,
      { value: id, label: selectedNames[id] ?? id },
    ])
  )
  for (const item of items) {
    if (item.enabled || selectedSet.has(item.id)) {
      options.set(item.id, {
        value: item.id,
        label: item.name,
        hint: item.enabled ? undefined : t('Disabled'),
      })
    }
  }
  const totalPages = groups.data?.page.totalPages ?? page
  return (
    <div className='space-y-2'>
      <Field>
        <FieldLabel htmlFor={fieldId}>{t('CPR account groups')}</FieldLabel>
        <MultiSelect
          id={fieldId}
          options={[...options.values()]}
          selected={props.selected}
          placeholder={t('Select groups')}
          disabled={props.disabled}
          onChange={(ids) => {
            setSelectedNames(
              Object.fromEntries(
                ids.map((id) => [id, options.get(id)?.label ?? id])
              )
            )
            props.onChange(ids)
          }}
        />
      </Field>
      {(page > 1 || totalPages > 1) && (
        <Pagination aria-label={t('Account group pages')}>
          <PaginationContent>
            <PaginationItem>
              <Button
                variant='outline'
                size='sm'
                disabled={page <= 1 || groups.isFetching}
                onClick={() => setPage(page - 1)}
              >
                {t('Previous')}
              </Button>
            </PaginationItem>
            <PaginationItem
              aria-live='polite'
              className='px-2 text-sm tabular-nums'
            >
              {t('Page {{page}} of {{total}}', {
                page: formatNumber(page, locale),
                total: formatNumber(totalPages, locale),
              })}
            </PaginationItem>
            <PaginationItem>
              <Button
                variant='outline'
                size='sm'
                disabled={page >= totalPages || groups.isFetching}
                onClick={() => setPage(page + 1)}
              >
                {t('Next')}
              </Button>
            </PaginationItem>
          </PaginationContent>
        </Pagination>
      )}
      {groups.isFetching && (
        <LoadingState inline size='sm' message={t('Loading groups…')} />
      )}
      {groups.isError && (
        <ErrorState
          title={t('Group options are unavailable.')}
          onRetry={() => void groups.refetch()}
          className='min-h-0'
        />
      )}
    </div>
  )
}
