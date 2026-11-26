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
import type { ColumnDef } from '@tanstack/react-table'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/ui/button'

import { AccountDetailsDialog } from './account-details-dialog'
import { getAccounts } from './api'
import type { AccountOAuthTarget } from './oauth-types'
import { ReadOnlyList } from './read-only-list'
import type { Account } from './types'

const STATUS_LABELS: Record<string, string> = {
  normal: 'Normal',
  quota_paused: 'Quota paused',
  quota_exhausted: 'Quota exhausted',
  rate_limited: 'Rate limited',
  disabled: 'Disabled',
  error: 'Error',
}

type AccountTableProps = {
  userId: number
  sessionId: string
  onAccessDenied: () => void
  onReauthorize: (target: AccountOAuthTarget) => void
}

export function AccountTable(props: AccountTableProps) {
  const { t } = useTranslation()
  const [details, setDetails] = useState<Account | null>(null)
  const columns: ColumnDef<Account>[] = [
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
      cell: ({ row }) => t(STATUS_LABELS[row.original.status] ?? 'Unknown'),
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
      id: 'details',
      header: t('Actions'),
      cell: ({ row }) => (
        <Button
          variant='outline'
          size='sm'
          onClick={() => setDetails(row.original)}
        >
          {t('Open account details')}
        </Button>
      ),
    },
  ]
  return (
    <>
      <ReadOnlyList
        kind='accounts'
        userId={props.userId}
        sessionId={props.sessionId}
        columns={columns}
        load={getAccounts}
        onAccessDenied={props.onAccessDenied}
      />
      {details && (
        <AccountDetailsDialog
          key={details.id}
          account={details}
          onClose={() => setDetails(null)}
          onReauthorize={(target) => {
            setDetails(null)
            props.onReauthorize(target)
          }}
        />
      )}
    </>
  )
}
