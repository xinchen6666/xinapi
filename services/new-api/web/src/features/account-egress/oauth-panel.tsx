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
import type { ColumnDef } from '@tanstack/react-table'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { DataTablePage, useDataTable } from '@/components/data-table'
import { ErrorState } from '@/components/error-state'
import { Button } from '@/components/ui/button'
import { formatTimestampToDate } from '@/lib/format'
import { getServerErrorStatus } from '@/lib/server-error-message'

import { listAccountOAuthOperations } from './oauth-api'
import { OAuthReceiptDialog } from './oauth-receipt-dialog'
import { OAuthStatus } from './oauth-receipt-facts'
import { OAuthStartDialog } from './oauth-start-dialog'
import type { AccountOAuthReceipt, AccountOAuthTarget } from './oauth-types'
import { TokenImportDialog } from './token-import-dialog'

const EMPTY_RECEIPTS: AccountOAuthReceipt[] = []
type Props = {
  userId: number
  sessionId: string
  startTarget: AccountOAuthTarget | 'new' | null
  onStart: () => void
  onCloseStart: () => void
  onAccessDenied: () => void
}

export function OAuthPanel(props: Props) {
  const { t } = useTranslation()
  const [importing, setImporting] = useState(false)
  const [active, setActive] = useState<{
    operationId: string
    authorizationUrl?: string
  } | null>(null)
  const query = useQuery({
    queryKey: [
      'account-egress',
      props.userId,
      'oauth',
      props.sessionId,
      'operations',
    ],
    queryFn: ({ signal }) => listAccountOAuthOperations(signal),
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const onAccessDenied = props.onAccessDenied
  useEffect(() => {
    if (getServerErrorStatus(query.error) === 403) onAccessDenied()
  }, [query.error, onAccessDenied])
  const columns: ColumnDef<AccountOAuthReceipt>[] = [
    {
      accessorKey: 'operation_id',
      header: t('Account name'),
      meta: { mobileTitle: true },
      cell: ({ row }) => (
        <Button
          variant='link'
          className='h-auto text-sm break-all whitespace-normal'
          onClick={() => setActive({ operationId: row.original.operation_id })}
        >
          {row.original.name || row.original.operation_id}
        </Button>
      ),
    },
    {
      accessorKey: 'provider',
      header: t('Provider'),
      cell: ({ row }) =>
        row.original.provider === 'openai' ? 'OpenAI' : 'xAI',
    },
    {
      accessorKey: 'status',
      header: t('Status'),
      cell: ({ row }) => <OAuthStatus status={row.original.status} />,
    },
    {
      accessorKey: 'account_id',
      header: t('Account ID'),
      cell: ({ row }) => (
        <span className='break-all'>
          {row.original.status === 'completed'
            ? row.original.account_id || '—'
            : '—'}
        </span>
      ),
    },
    {
      accessorKey: 'expires_at',
      header: t('Expires at'),
      cell: ({ row }) =>
        formatTimestampToDate(
          Date.parse(row.original.expires_at),
          'milliseconds'
        ),
    },
    {
      id: 'actions',
      header: t('Actions'),
      cell: ({ row }) => (
        <Button
          variant='outline'
          size='sm'
          onClick={() => setActive({ operationId: row.original.operation_id })}
        >
          {t('View receipt')}
        </Button>
      ),
    },
  ]
  const { table } = useDataTable({
    data: query.isError ? EMPTY_RECEIPTS : (query.data ?? EMPTY_RECEIPTS),
    columns,
    getRowId: (row) => row.operation_id,
    enableSorting: false,
    enableRowSelection: false,
  })
  return (
    <>
      <p className='text-muted-foreground text-sm'>
        {t(
          'Choose official login or import an existing access token. Both methods use your approved node.'
        )}
      </p>
      {query.isError ? (
        <ErrorState
          title={t('Account authorization service is unavailable')}
          onRetry={() => void query.refetch()}
        />
      ) : (
        <DataTablePage
          table={table}
          columns={columns}
          isLoading={query.isPending}
          isFetching={query.isFetching}
          emptyTitle={t('Start an authorization to create a session receipt')}
          mobileProps={{ enableRowSelection: false }}
          toolbar={
            <div className='flex flex-wrap justify-end gap-2'>
              <Button
                variant='outline'
                size='sm'
                disabled={query.isFetching}
                onClick={() => void query.refetch()}
              >
                {t('Refresh receipts')}
              </Button>
              <Button
                size='sm'
                variant='outline'
                onClick={() => setImporting(true)}
              >
                {t('Import access token')}
              </Button>
              <Button size='sm' onClick={props.onStart}>
                {t('Official login')}
              </Button>
            </div>
          }
        />
      )}
      {importing && (
        <TokenImportDialog
          userId={props.userId}
          sessionId={props.sessionId}
          onClose={() => {
            setImporting(false)
            void query.refetch()
          }}
          onAccessDenied={props.onAccessDenied}
          onImported={(receipt) => {
            setImporting(false)
            setActive({ operationId: receipt.operation_id })
            void query.refetch()
          }}
        />
      )}
      {props.startTarget && (
        <OAuthStartDialog
          userId={props.userId}
          sessionId={props.sessionId}
          target={props.startTarget === 'new' ? undefined : props.startTarget}
          onClose={props.onCloseStart}
          onAccessDenied={props.onAccessDenied}
          onReadReceipts={() => {
            props.onCloseStart()
            void query.refetch()
          }}
          onStarted={(receipt) => {
            setActive({
              operationId: receipt.operation_id,
              authorizationUrl: receipt.authorization_url,
            })
            props.onCloseStart()
            void query.refetch()
          }}
        />
      )}
      {active && (
        <OAuthReceiptDialog
          key={active.operationId}
          userId={props.userId}
          sessionId={props.sessionId}
          operationId={active.operationId}
          authorizationUrl={active.authorizationUrl}
          onClose={() => {
            setActive(null)
            void query.refetch()
          }}
          onAccessDenied={props.onAccessDenied}
          onNewAuthorization={() => {
            setActive(null)
            props.onStart()
          }}
        />
      )}
    </>
  )
}
