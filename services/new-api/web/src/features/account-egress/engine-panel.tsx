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
import type { ColumnDef } from '@tanstack/react-table'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ConfirmDialog } from '@/components/confirm-dialog'
import { DataTablePage, useDataTable } from '@/components/data-table'
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
import { formatBytes, formatNumber } from '@/lib/format'
import { getServerErrorMessage } from '@/lib/server-error-message'
import { useAuthStore } from '@/stores/auth-store'

import {
  getEngines,
  reclaimReservation,
  stopEngine,
  type EngineEntry,
} from './console-api'

const noEntries: EngineEntry[] = []
type SlotAction = { kind: 'stop' | 'reclaim'; id: string }

export function EnginePanel(props: { occupantsOnly?: boolean }) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const user = useAuthStore((state) => state.auth.user)
  const sessionId = useAuthStore((state) => state.auth.session?.sid)
  const queryClient = useQueryClient()
  const [action, setAction] = useState<SlotAction | null>(null)
  const canManage = hasPermission(
    user,
    ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
    ADMIN_PERMISSION_ACTIONS.MANAGE_EGRESS
  )
  const inventory = useQuery({
    queryKey: ['account-egress', user?.id, 'engines', sessionId],
    queryFn: ({ signal }) => getEngines(signal),
    enabled: user != null,
    retry: false,
    gcTime: 0,
    refetchInterval: 6000,
    meta: { errorToast: false },
  })
  const release = useMutation({
    mutationFn: async (item: SlotAction): Promise<void> => {
      if (item.kind === 'stop') await stopEngine(item.id)
      else await reclaimReservation(item.id)
    },
    onSuccess: () => setAction(null),
    onSettled: () =>
      queryClient.invalidateQueries({ queryKey: ['account-egress', user?.id] }),
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })

  const stateLabel = (state: string): string => {
    switch (state) {
      case 'starting':
        return t('Starting')
      case 'running':
        return t('Running')
      case 'stopped':
        return t('Stopped')
      case 'failed':
        return t('Failed')
      default:
        return t('Unknown')
    }
  }
  const columns: ColumnDef<EngineEntry>[] = [
    {
      id: 'owner',
      header: t('Slot owner'),
      meta: { mobileTitle: true },
      cell: ({ row }) => {
        const owner = row.original.owner
        if (!owner) return t('Unassigned')
        return (
          <div className='flex flex-col gap-1'>
            <span>
              {owner.kind === 'account'
                ? t('Upstream account')
                : t('Reservation')}
            </span>
            <code className='text-muted-foreground text-xs break-all'>
              {owner.id}
            </code>
          </div>
        )
      },
    },
    { accessorKey: 'node_name', header: t('Node') },
    {
      id: 'state',
      header: t('Status'),
      cell: ({ row }) => (
        <div className='flex flex-wrap gap-1'>
          <Badge variant='outline'>{stateLabel(row.original.state)}</Badge>
          {row.original.occupies_slot && (
            <Badge variant='secondary'>{t('Occupies a slot')}</Badge>
          )}
        </div>
      ),
    },
    {
      id: 'resources',
      header: t('Resources'),
      cell: ({ row }) => (
        <div className='text-muted-foreground flex flex-col text-xs'>
          {row.original.rss_bytes != null && (
            <span>
              {t('Resident memory')}:{' '}
              {formatBytes(row.original.rss_bytes, locale)}
            </span>
          )}
          {row.original.cpu_seconds != null && (
            <span>
              {t('Cumulative CPU seconds')}:{' '}
              {formatNumber(row.original.cpu_seconds, locale)}
            </span>
          )}
          {row.original.cold_start_ms != null && (
            <span>
              {t('Cold start (ms)')}:{' '}
              {formatNumber(row.original.cold_start_ms, locale)}
            </span>
          )}
          {row.original.expires_at && (
            <time dateTime={row.original.expires_at}>
              {t('Expires at')}:{' '}
              {new Date(row.original.expires_at).toLocaleString(locale)}
            </time>
          )}
        </div>
      ),
    },
  ]
  if (canManage) {
    columns.push({
      id: 'actions',
      header: t('Actions'),
      cell: ({ row }) => {
        const engine = row.original
        let item: SlotAction
        if (engine.stoppable && engine.owner?.kind === 'account') {
          item = { kind: 'stop', id: engine.entry_id }
        } else if (engine.reclaimable && engine.owner?.kind === 'reservation') {
          item = { kind: 'reclaim', id: engine.owner.id }
        } else {
          return null
        }
        return (
          <Button
            size='sm'
            variant='outline'
            disabled={release.isPending}
            onClick={() => {
              release.reset()
              setAction(item)
            }}
          >
            {item.kind === 'stop' ? t('Stop engine') : t('Reclaim reservation')}
          </Button>
        )
      },
    })
  }
  const data = inventory.data
  const { table } = useDataTable({
    data: props.occupantsOnly
      ? (data?.items.filter((entry) => entry.occupies_slot) ?? noEntries)
      : (data?.items ?? noEntries),
    columns,
    getRowId: (entry) => entry.entry_id,
    enableSorting: false,
    enableRowSelection: false,
  })
  if (inventory.isError) {
    return (
      <ErrorState
        title={t('Engine inventory is unavailable')}
        description={getServerErrorMessage(inventory.error)}
        onRetry={() => void inventory.refetch()}
      />
    )
  }
  if (!data) return <LoadingState />
  return (
    <div className='flex min-w-0 flex-col gap-3'>
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <div role='status' className='flex flex-wrap items-center gap-2'>
          <span>
            {t('Engine slots: {{used}} of {{limit}} in use', {
              used: formatNumber(data.capacity.used, locale),
              limit: formatNumber(data.capacity.limit, locale),
            })}
          </span>
          {data.capacity.available === 0 && (
            <Badge variant='destructive'>{t('Engine slots are full')}</Badge>
          )}
        </div>
        <Button
          size='sm'
          variant='outline'
          disabled={inventory.isFetching}
          onClick={() => void inventory.refetch()}
        >
          {t('Refresh')}
        </Button>
      </div>
      <DataTablePage
        table={table}
        columns={columns}
        fixedHeight={false}
        emptyTitle={t('No engine entries')}
      />
      {release.isError && (
        <p role='alert' className='text-destructive text-sm'>
          {getServerErrorMessage(release.error)}
        </p>
      )}
      {action && (
        <ConfirmDialog
          open
          destructive
          onOpenChange={(open) => {
            if (!open && !release.isPending) setAction(null)
          }}
          title={
            action.kind === 'stop' ? t('Stop engine') : t('Reclaim reservation')
          }
          desc={
            action.kind === 'stop'
              ? t(
                  'Stopping the engine closes existing connections. The account stays bound and pauses new sends.'
                )
              : t('Release the expired reservation and its engine slot.')
          }
          isLoading={release.isPending}
          handleConfirm={() => release.mutate(action)}
        />
      )}
    </div>
  )
}
