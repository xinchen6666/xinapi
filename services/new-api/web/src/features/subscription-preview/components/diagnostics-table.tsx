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
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { DataTablePage, useDataTable } from '@/components/data-table'

import {
  buildDiagnosticRows,
  diagnosticMessage,
} from '../lib/subscription-preview'
import type { DiagnosticRow, SubscriptionPreviewDiagnostic } from '../types'

type DiagnosticsTableProps = {
  diagnostics: SubscriptionPreviewDiagnostic[]
}

export function DiagnosticsTable(props: DiagnosticsTableProps) {
  const { t } = useTranslation()
  const rows = useMemo(
    () => buildDiagnosticRows(props.diagnostics),
    [props.diagnostics]
  )

  const columns = useMemo<ColumnDef<DiagnosticRow, unknown>[]>(
    () => [
      {
        accessorKey: 'code',
        header: t('Code'),
        size: 220,
        meta: { mobileTitle: true },
        cell: ({ row }) => (
          <span className='font-mono break-all'>{row.original.code}</span>
        ),
      },
      {
        accessorKey: 'subject',
        header: t('Subject'),
        size: 200,
        cell: ({ row }) => (
          <span className='break-all'>{row.original.subject || '—'}</span>
        ),
      },
      {
        accessorKey: 'message',
        header: t('Message'),
        cell: ({ row }) => (
          <span className='break-words whitespace-normal'>
            {diagnosticMessage(row.original, t)}
          </span>
        ),
      },
    ],
    [t]
  )

  const { table } = useDataTable({
    columns,
    data: rows,
    getRowId: (row) => row.id,
    enableSorting: false,
    enableRowSelection: false,
    withPaginationRowModel: false,
  })

  return (
    <DataTablePage
      table={table}
      columns={columns}
      fixedHeight={false}
      showPagination={false}
      emptyTitle={t('No issues were reported')}
    />
  )
}
