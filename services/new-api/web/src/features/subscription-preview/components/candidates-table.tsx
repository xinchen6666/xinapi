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
import type {
  ColumnDef,
  ColumnFiltersState,
  OnChangeFn,
  RowSelectionState,
} from '@tanstack/react-table'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { DataTablePage, useDataTable } from '@/components/data-table'
import { StatusBadge } from '@/components/status-badge'
import { Checkbox } from '@/components/ui/checkbox'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber } from '@/lib/format'

import { groupCandidateRows } from '../lib/subscription-preview'
import type { CandidateRow } from '../types'

const ALL_PROTOCOLS = 'all'

type CandidatesTableProps = {
  rows: CandidateRow[]
  enableSelection: boolean
  rowSelection: RowSelectionState
  onRowSelectionChange: OnChangeFn<RowSelectionState>
}

export function CandidatesTable(props: CandidatesTableProps) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const [protocol, setProtocol] = useState(ALL_PROTOCOLS)
  const groups = useMemo(() => groupCandidateRows(props.rows), [props.rows])
  const orderedRows = useMemo(
    () => groups.flatMap((group) => group.rows),
    [groups]
  )
  const activeProtocol = groups.some((group) => group.protocol === protocol)
    ? protocol
    : ALL_PROTOCOLS
  const columnFilters = useMemo<ColumnFiltersState>(
    () =>
      activeProtocol === ALL_PROTOCOLS
        ? []
        : [{ id: 'protocol', value: activeProtocol }],
    [activeProtocol]
  )

  const columns = useMemo<ColumnDef<CandidateRow, unknown>[]>(() => {
    const selection: ColumnDef<CandidateRow, unknown>[] = props.enableSelection
      ? [
          {
            id: 'select',
            header: ({ table }) => (
              <Checkbox
                checked={table.getIsAllPageRowsSelected()}
                indeterminate={table.getIsSomePageRowsSelected()}
                disabled={
                  !table.getRowModel().rows.some((row) => row.getCanSelect())
                }
                onCheckedChange={(value) =>
                  table.toggleAllPageRowsSelected(Boolean(value))
                }
                aria-label={t('Select all')}
              />
            ),
            cell: ({ row }) => (
              <Checkbox
                checked={row.getIsSelected()}
                disabled={!row.getCanSelect()}
                onCheckedChange={(value) => row.toggleSelected(Boolean(value))}
                aria-label={t('Select {{name}}', {
                  name: row.original.name || '—',
                })}
              />
            ),
            size: 40,
            enableSorting: false,
            enableHiding: false,
          },
        ]
      : []
    return [
      ...selection,
      {
        accessorKey: 'name',
        header: t('Name'),
        size: 300,
        meta: { mobileTitle: true },
        cell: ({ row }) => (
          <span className='break-all'>{row.original.name || '—'}</span>
        ),
      },
      {
        accessorKey: 'protocol',
        header: t('Protocol'),
        filterFn: 'equalsString',
        size: 120,
        cell: ({ row }) => (
          <StatusBadge
            variant='neutral'
            label={row.original.protocol}
            copyable={false}
          />
        ),
      },
      {
        accessorKey: 'parse_valid',
        header: t('Parse result'),
        size: 160,
        meta: { mobileBadge: true },
        cell: ({ row }) => (
          <StatusBadge
            variant={row.original.parse_valid ? 'success' : 'neutral'}
            label={
              row.original.parse_valid
                ? t('Parse valid')
                : t('Needs correction')
            }
            copyable={false}
          />
        ),
      },
      {
        accessorKey: 'supported',
        header: t('Transport'),
        size: 180,
        cell: ({ row }) => {
          let label = t('Skipped')
          if (row.original.parse_valid) {
            label = row.original.supported
              ? t('Native HTTP/SOCKS')
              : t('Mihomo pending')
          }
          return (
            <StatusBadge variant='neutral' label={label} copyable={false} />
          )
        },
      },
    ]
  }, [props.enableSelection, t])

  const { table } = useDataTable({
    columns,
    data: orderedRows,
    getRowId: (row) => row.id,
    enableSorting: false,
    enableRowSelection: (row) =>
      props.enableSelection && row.original.parse_valid,
    rowSelection: props.rowSelection,
    onRowSelectionChange: props.onRowSelectionChange,
    columnFilters,
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
        >{`${t('All')} (${formatNumber(orderedRows.length, locale)})`}</TabsTrigger>
        {groups.map((group) => (
          <TabsTrigger key={group.protocol} value={group.protocol}>
            {`${group.protocol} (${formatNumber(group.rows.length, locale)})`}
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
      mobileProps={{ enableRowSelection: props.enableSelection }}
      fixedHeight={false}
      emptyTitle={t('The preview found no nodes in this subscription')}
    />
  )
}
