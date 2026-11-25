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
import { isAxiosError } from 'axios'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ErrorState } from '@/components/error-state'
import { LoadingState } from '@/components/loading-state'
import { Button } from '@/components/ui/button'
import { FieldError } from '@/components/ui/field'
import { ROLE } from '@/lib/roles'
import { getServerErrorMessage } from '@/lib/server-error-message'
import { useAuthStore } from '@/stores/auth-store'

import {
  approveSource,
  listSources,
  refreshSource,
  sourceDetail,
} from '../source-api'
import type { SourceDetail } from '../source-types'
import { SourceEditor } from './source-editor'
import { SourceReviewPanel } from './source-review'
import { ApprovedNodes, SourcesTable } from './sources-table'

export function SourceManagement(props: { onAccessDenied: () => void }) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()
  const user = useAuthStore((state) => state.auth.user)
  const canWrite = !!user && user.role >= ROLE.SUPER_ADMIN
  const [sourceId, setSourceId] = useState('')
  const [editor, setEditor] = useState<'create' | 'update' | null>(null)
  const sources = useQuery({
    queryKey: ['subscription-sources', user?.id],
    queryFn: listSources,
    retry: false,
    meta: { errorToast: false },
  })
  const detailKey = ['subscription-source-detail', user?.id, sourceId]
  const detail = useQuery({
    queryKey: detailKey,
    queryFn: () => sourceDetail(sourceId),
    enabled: !!sourceId,
    retry: false,
    meta: { errorToast: false },
  })
  const updated = (value: SourceDetail) => {
    setSourceId(value.source.id)
    queryClient.setQueryData(
      ['subscription-source-detail', user?.id, value.source.id],
      value
    )
    void queryClient.invalidateQueries({
      queryKey: ['subscription-sources', user?.id],
    })
  }
  const failed = (error: unknown) => {
    if (isAxiosError(error) && error.response?.status === 409) {
      void queryClient.invalidateQueries({ queryKey: detailKey })
      void queryClient.invalidateQueries({
        queryKey: ['subscription-sources', user?.id],
      })
    }
    if (isAxiosError(error) && error.response?.status === 403) {
      props.onAccessDenied()
    }
  }
  const refresh = useMutation({
    mutationFn: refreshSource,
    onSuccess: updated,
    onError: failed,
    meta: { errorToast: false },
  })
  const approve = useMutation({
    mutationFn: approveSource,
    onSuccess: updated,
    onError: failed,
    meta: { errorToast: false },
  })
  const busy = refresh.isPending || approve.isPending
  const operationError = refresh.error || approve.error
  if (sources.isLoading) return <LoadingState />
  if (sources.isError) {
    return (
      <ErrorState
        title={t('Source list failed')}
        description={getServerErrorMessage(sources.error)}
        onRetry={() => void sources.refetch()}
      />
    )
  }
  return (
    <div className='space-y-4'>
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <p className='text-muted-foreground text-sm'>
          {t('Sources save approved versions and stable node identities.')}
        </p>
        <div className='flex flex-wrap gap-2'>
          <Button variant='outline' onClick={() => void sources.refetch()}>
            {t('Reload source list')}
          </Button>
          {canWrite ? (
            <Button onClick={() => setEditor('create')}>
              {t('Create subscription source')}
            </Button>
          ) : null}
        </div>
      </div>
      {!canWrite ? (
        <p className='rounded-lg border p-3 text-sm'>
          {t(
            'Administrators can view sources. Super administrators can manage sources and approve versions.'
          )}
        </p>
      ) : null}
      <SourcesTable
        items={sources.data ?? []}
        onSelect={(source) => {
          refresh.reset()
          approve.reset()
          setSourceId(source.id)
        }}
      />
      {detail.isLoading ? <LoadingState /> : null}
      {detail.isError ? (
        <ErrorState
          title={t('Source details failed')}
          description={getServerErrorMessage(detail.error)}
          onRetry={() => void detail.refetch()}
        />
      ) : null}
      {detail.data ? (
        <section className='space-y-4'>
          <div className='flex flex-wrap items-center justify-between gap-2'>
            <h2 className='text-lg font-semibold break-all'>
              {detail.data.source.name}
            </h2>
            <div className='flex flex-wrap gap-2'>
              <Button
                variant='outline'
                disabled={busy}
                onClick={() => {
                  refresh.reset()
                  approve.reset()
                  void detail.refetch()
                }}
              >
                {t('Reload details')}
              </Button>
              {canWrite ? (
                <>
                  <Button
                    variant='outline'
                    disabled={busy}
                    onClick={() => setEditor('update')}
                  >
                    {t('Edit subscription source')}
                  </Button>
                  <Button
                    disabled={busy || detail.isFetching}
                    onClick={() => {
                      approve.reset()
                      refresh.mutate({
                        source_id: detail.data.source.id,
                        expected_revision: detail.data.source.revision,
                      })
                    }}
                  >
                    {t('Refresh source')}
                  </Button>
                </>
              ) : null}
            </div>
          </div>
          {detail.data.source.last_error_code ? (
            <p role='status' className='rounded-lg border p-3 text-sm'>
              {t('Refresh needs attention. Approved nodes remain available.')}{' '}
              <span className='font-mono'>
                {detail.data.source.last_error_code}
              </span>
            </p>
          ) : null}
          {operationError ? (
            <FieldError>{getServerErrorMessage(operationError)}</FieldError>
          ) : null}
          <h3 className='font-semibold'>{t('Approved nodes')}</h3>
          <ApprovedNodes nodes={detail.data.nodes} />
          {detail.data.review ? (
            <SourceReviewPanel
              key={`${detail.data.review.id}:${detail.data.source.revision}`}
              detail={detail.data}
              canWrite={canWrite}
              pending={busy || detail.isFetching}
              onApprove={(input) => {
                refresh.reset()
                approve.mutate(input)
              }}
            />
          ) : null}
        </section>
      ) : null}
      {editor ? (
        <SourceEditor
          key={`${editor}:${detail.data?.source.id ?? 'new'}`}
          detail={editor === 'update' ? detail.data : undefined}
          onClose={() => setEditor(null)}
          onSaved={(value) => {
            updated(value)
            setEditor(null)
          }}
          onConflict={() => void detail.refetch()}
        />
      ) : null}
    </div>
  )
}
