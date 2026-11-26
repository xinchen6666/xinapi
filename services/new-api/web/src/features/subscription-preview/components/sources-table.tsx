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
import { Button } from '@/components/ui/button'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber } from '@/lib/format'

import type { SourceNode, SourceSummary } from '../source-types'

type Props = {
  items: SourceSummary[]
  onSelect: (source: SourceSummary) => void
}
export function SourcesTable(props: Props) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const columns = useMemo<ColumnDef<SourceSummary, unknown>[]>(
    () => [
      {
        accessorKey: 'name',
        header: t('Source name'),
        meta: { mobileTitle: true },
        cell: ({ row }) => (
          <Button
            variant='link'
            className='h-auto break-all whitespace-normal'
            onClick={() => props.onSelect(row.original)}
          >
            {row.original.name}
          </Button>
        ),
      },
      {
        accessorKey: 'status',
        header: t('Status'),
        cell: ({ row }) =>
          t(`Source status: ${row.original.status}`, {
            defaultValue: t('Source status: pending'),
          }),
      },
      {
        accessorKey: 'revision',
        header: t('Revision'),
        cell: ({ row }) => formatNumber(row.original.revision, locale),
      },
      {
        accessorKey: 'approved_version',
        header: t('Approved version'),
        cell: ({ row }) => formatNumber(row.original.approved_version, locale),
      },
      {
        accessorKey: 'node_count',
        header: t('Approved nodes'),
        cell: ({ row }) => formatNumber(row.original.node_count, locale),
      },
      {
        accessorKey: 'has_pending_review',
        header: t('Pending review'),
        cell: ({ row }) =>
          row.original.has_pending_review ? t('Yes') : t('—'),
      },
      { accessorKey: 'updated_at', header: t('Updated at') },
    ],
    [t, locale, props]
  )
  const { table } = useDataTable({
    data: props.items,
    columns,
    getRowId: (row) => row.id,
    enableSorting: false,
    enableRowSelection: false,
  })
  return (
    <DataTablePage
      table={table}
      columns={columns}
      fixedHeight={false}
      emptyTitle={t('Create a source to save approved node versions')}
    />
  )
}
export function ApprovedNodes(props: {
  nodes: SourceNode[]
  onVerify?: (node: SourceNode) => void
}) {
  const { t } = useTranslation()
  const onVerify = props.onVerify
  const columns = useMemo<ColumnDef<SourceNode, unknown>[]>(
    () => [
      { accessorKey: 'name', header: t('Name'), meta: { mobileTitle: true } },
      {
        accessorKey: 'id',
        header: t('Stable node ID'),
        cell: ({ row }) => (
          <span className='font-mono text-xs break-all'>{row.original.id}</span>
        ),
      },
      { accessorKey: 'protocol', header: t('Protocol') },
      {
        accessorKey: 'version_id',
        header: t('Connection version'),
        cell: ({ row }) => (
          <span className='font-mono text-xs break-all'>
            {row.original.version_id}
          </span>
        ),
      },
      {
        accessorKey: 'available',
        header: t('Availability'),
        cell: ({ row }) =>
          row.original.available ? t('Available') : t('Needs refresh'),
      },
      {
        accessorKey: 'supported',
        header: t('Transport'),
        cell: ({ row }) =>
          row.original.supported ? t('Native HTTP/SOCKS') : t('Mihomo pending'),
      },
      ...(onVerify
        ? [
            {
              id: 'verify',
              header: t('Actions'),
              cell: ({ row }: { row: { original: SourceNode } }) => (
                <Button
                  size='sm'
                  variant='outline'
                  disabled={
                    !row.original.available ||
                    !row.original.supported ||
                    !row.original.version_id
                  }
                  onClick={() => onVerify(row.original)}
                >
                  {t('Verify connection')}
                </Button>
              ),
            },
          ]
        : []),
    ],
    [t, onVerify]
  )
  const { table } = useDataTable({
    data: props.nodes ?? [],
    columns,
    getRowId: (row) => row.id,
    enableSorting: false,
    enableRowSelection: false,
  })
  return (
    <DataTablePage
      table={table}
      columns={columns}
      fixedHeight={false}
      emptyTitle={t('Approve a review to create node versions')}
    />
  )
}
