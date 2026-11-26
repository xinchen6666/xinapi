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
import type { ColumnDef, PaginationState } from '@tanstack/react-table'
import { isAxiosError } from 'axios'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { DataTablePage, useDataTable } from '@/components/data-table'
import { ErrorState } from '@/components/error-state'
import { Button } from '@/components/ui/button'

import type { ListPage } from './types'

const EMPTY_ITEMS: never[] = []

type ReadOnlyListProps<T extends { id: string }> = {
  kind: 'accounts' | 'proxies'
  userId: number
  sessionId: string
  columns: ColumnDef<T, unknown>[]
  load: (
    page: number,
    pageSize: number,
    signal: AbortSignal
  ) => Promise<ListPage<T>>
  onAccessDenied: () => void
}

export function ReadOnlyList<T extends { id: string }>(
  props: ReadOnlyListProps<T>
) {
  const { t } = useTranslation()
  const [pagination, setPagination] = useState<PaginationState>({
    pageIndex: 0,
    pageSize: 20,
  })
  const query = useQuery({
    queryKey: [
      'account-egress',
      props.userId,
      props.kind,
      props.sessionId,
      pagination,
    ],
    queryFn: ({ signal }) =>
      props.load(pagination.pageIndex + 1, pagination.pageSize, signal),
    retry: false,
    gcTime: 0,
  })
  const accessDenied =
    isAxiosError(query.error) && query.error.response?.status === 403
  const onAccessDenied = props.onAccessDenied
  useEffect(() => {
    if (accessDenied) onAccessDenied()
  }, [accessDenied, onAccessDenied])
  const { table } = useDataTable({
    columns: props.columns,
    data: query.isError ? EMPTY_ITEMS : (query.data?.items ?? EMPTY_ITEMS),
    totalCount: query.isError ? 0 : (query.data?.page.total ?? 0),
    pageCount: query.isError ? 0 : (query.data?.page.totalPages ?? 0),
    getRowId: (item) => item.id,
    pagination,
    onPaginationChange: (updater) => {
      if (query.isFetching || query.isError) return
      setPagination((current) => {
        const next = typeof updater === 'function' ? updater(current) : updater
        return {
          ...next,
          pageIndex: current.pageSize === next.pageSize ? next.pageIndex : 0,
        }
      })
    },
    manualPagination: true,
    enableSorting: false,
    enableRowSelection: false,
  })
  if (query.isError) {
    return (
      <ErrorState
        title={t('Account and egress service is unavailable')}
        onRetry={() => void query.refetch()}
      />
    )
  }
  return (
    <DataTablePage
      table={table}
      columns={props.columns}
      isLoading={query.isPending}
      isFetching={query.isFetching}
      emptyTitle={t('No records')}
      toolbar={
        <div className='flex justify-end'>
          <Button
            variant='outline'
            size='sm'
            disabled={query.isFetching}
            onClick={() => void query.refetch()}
          >
            {t('Refresh')}
          </Button>
        </div>
      }
      mobileProps={{ enableRowSelection: false }}
      applyHeaderSize
    />
  )
}
