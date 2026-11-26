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
import { useQuery } from '@tanstack/react-query'
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ConfirmDialog } from '@/components/confirm-dialog'
import { MultiSelect } from '@/components/multi-select'
import { Button } from '@/components/ui/button'
import { Field, FieldLabel } from '@/components/ui/field'
import {
  Pagination,
  PaginationContent,
  PaginationItem,
} from '@/components/ui/pagination'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber } from '@/lib/format'

import { getAccountGroups, type AccountGroup } from './console-api'

type AccountGroupEditorProps = {
  userId: number
  sessionId: string
  initialGroups: AccountGroup[]
  pending: boolean
  needsRefresh: boolean
  message?: string
  onClose: () => void
  onSave: (selectedIds: string[], expectedIds: string[]) => void
  onReload: () => Promise<AccountGroup[] | null>
}

export function AccountGroupEditor(props: AccountGroupEditorProps) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const fieldId = useId()
  const [expected, setExpected] = useState(props.initialGroups)
  const [selected, setSelected] = useState(props.initialGroups)
  const [page, setPage] = useState(1)
  const [reloading, setReloading] = useState(false)
  const groups = useQuery({
    queryKey: [
      'account-egress',
      props.userId,
      'account-groups',
      props.sessionId,
      page,
    ],
    queryFn: ({ signal }) => getAccountGroups(page, signal),
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const busy = props.pending || reloading
  const expectedIds = new Set(expected.map((group) => group.id))
  const selectedIds = selected.map((group) => group.id)
  const changed =
    selectedIds.length !== expectedIds.size ||
    selectedIds.some((id) => !expectedIds.has(id))
  const byId = new Map(expected.map((group) => [group.id, group]))
  for (const group of selected) byId.set(group.id, group)
  for (const group of groups.data?.items ?? []) {
    if (group.enabled || byId.has(group.id)) byId.set(group.id, group)
  }
  const options = Array.from(byId.values(), (group) => ({
    value: group.id,
    label: group.name,
    hint: group.enabled === false ? t('Disabled') : undefined,
  }))
  const totalPages = groups.data?.page.totalPages ?? page

  const reload = async () => {
    setReloading(true)
    try {
      const latest = await props.onReload()
      if (latest) {
        setExpected(latest)
        setSelected(latest)
      }
    } finally {
      setReloading(false)
    }
  }

  return (
    <ConfirmDialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) props.onClose()
      }}
      title={t('Edit account groups')}
      desc={t('Review the complete group assignment before saving.')}
      confirmText={t('Save changes')}
      disabled={!changed || props.needsRefresh}
      isLoading={busy}
      handleConfirm={() => {
        if (!busy && changed && !props.needsRefresh) {
          props.onSave(selectedIds, [...expectedIds])
        }
      }}
    >
      <div className='space-y-3'>
        <Field>
          <FieldLabel htmlFor={fieldId}>{t('Groups')}</FieldLabel>
          <MultiSelect
            id={fieldId}
            options={options}
            selected={selectedIds}
            onChange={(ids) => {
              setSelected(
                ids.flatMap((id) => {
                  const group = byId.get(id)
                  return group ? [group] : []
                })
              )
            }}
            placeholder={t('Select groups')}
            disabled={busy || props.needsRefresh}
          />
        </Field>
        {(page > 1 || totalPages > 1) && (
          <Pagination aria-label={t('Account group pages')}>
            <PaginationContent>
              <PaginationItem>
                <Button
                  variant='outline'
                  size='sm'
                  disabled={busy || page <= 1 || groups.isFetching}
                  onClick={() => setPage(page - 1)}
                >
                  {t('Previous')}
                </Button>
              </PaginationItem>
              <PaginationItem
                className='px-2 text-sm tabular-nums'
                aria-live='polite'
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
                  disabled={busy || page >= totalPages || groups.isFetching}
                  onClick={() => setPage(page + 1)}
                >
                  {t('Next')}
                </Button>
              </PaginationItem>
            </PaginationContent>
          </Pagination>
        )}
        {groups.isFetching && (
          <p role='status' className='text-muted-foreground text-sm'>
            {t('Loading groups…')}
          </p>
        )}
        {groups.isError && (
          <div className='flex flex-wrap items-center gap-2'>
            <p role='alert' className='text-destructive text-sm'>
              {t('Group options are unavailable.')}
            </p>
            <Button
              variant='outline'
              size='sm'
              onClick={() => void groups.refetch()}
            >
              {t('Refresh')}
            </Button>
          </div>
        )}
        {props.message && (
          <p className='text-destructive text-sm' role='alert'>
            {props.message}
          </p>
        )}
        {props.needsRefresh && (
          <Button
            variant='outline'
            size='sm'
            disabled={busy}
            onClick={() => void reload()}
          >
            {t('Reload account groups')}
          </Button>
        )}
      </div>
    </ConfirmDialog>
  )
}
