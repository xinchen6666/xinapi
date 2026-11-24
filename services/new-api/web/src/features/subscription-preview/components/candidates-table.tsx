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
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { DataTablePage, useDataTable } from '@/components/data-table'
import { StatusBadge } from '@/components/status-badge'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'

import {
  buildCandidateRows,
  groupCandidateRows,
} from '../lib/subscription-preview'
import type { CandidateRow, SubscriptionPreviewCandidate } from '../types'

const ALL_PROTOCOLS = 'all'

type CandidatesTableProps = {
  candidates: SubscriptionPreviewCandidate[]
}

export function CandidatesTable(props: CandidatesTableProps) {
  const { t } = useTranslation()
  const [protocol, setProtocol] = useState(ALL_PROTOCOLS)
  const groups = useMemo(
    () => groupCandidateRows(buildCandidateRows(props.candidates)),
    [props.candidates]
  )
  const orderedRows = useMemo(
    () => groups.flatMap((group) => group.rows),
    [groups]
  )
  // A fresh preview can drop the protocol the tab was showing.
  const activeProtocol = groups.some((group) => group.protocol === protocol)
    ? protocol
    : ALL_PROTOCOLS
  const activeRows =
    activeProtocol === ALL_PROTOCOLS
      ? orderedRows
      : (groups.find((group) => group.protocol === activeProtocol)?.rows ?? [])

  const columns = useMemo<ColumnDef<CandidateRow, unknown>[]>(
    () => [
      {
        accessorKey: 'name',
        header: t('Name'),
        size: 320,
        meta: { mobileTitle: true },
        cell: ({ row }) => (
          <span className='break-all'>{row.original.name || '—'}</span>
        ),
      },
      {
        accessorKey: 'protocol',
        header: t('Protocol'),
        size: 140,
        cell: ({ row }) => (
          <StatusBadge
            variant='neutral'
            label={row.original.protocol}
            copyable={false}
          />
        ),
      },
      {
        accessorKey: 'supported',
        header: t('Available'),
        size: 120,
        meta: { mobileBadge: true },
        cell: ({ row }) => (
          <StatusBadge
            variant={row.original.supported ? 'success' : 'neutral'}
            label={row.original.supported ? t('Yes') : t('No')}
            copyable={false}
          />
        ),
      },
    ],
    [t]
  )

  const { table } = useDataTable({
    columns,
    data: activeRows,
    getRowId: (row) => row.id,
    enableSorting: false,
    enableRowSelection: false,
  })

  const toolbar = (
    <Tabs
      value={activeProtocol}
      onValueChange={(value) =>
        setProtocol(typeof value === 'string' ? value : ALL_PROTOCOLS)
      }
    >
      <TabsList
        aria-label={t('Protocol')}
        className='max-w-full overflow-x-auto'
      >
        <TabsTrigger
          value={ALL_PROTOCOLS}
        >{`${t('All')} (${orderedRows.length})`}</TabsTrigger>
        {groups.map((group) => (
          <TabsTrigger key={group.protocol} value={group.protocol}>
            {`${group.protocol} (${group.rows.length})`}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  )

  return (
    <DataTablePage
      table={table}
      columns={columns}
      toolbar={toolbar}
      fixedHeight={false}
      emptyTitle={t('The preview found no nodes in this subscription')}
    />
  )
}
