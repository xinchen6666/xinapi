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
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { Dialog } from '@/components/dialog'
import { ErrorState } from '@/components/error-state'
import { LoadingState } from '@/components/loading-state'
import { Button } from '@/components/ui/button'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber } from '@/lib/format'
import {
  getServerErrorMessage,
  getServerErrorStatus,
} from '@/lib/server-error-message'
import { useAuthStore } from '@/stores/auth-store'

import {
  testApprovedNode,
  isEgressCapacityError,
  type ApprovedNodeTarget,
} from './console-api'
import { EnginePanel } from './engine-panel'

type TestTarget = ApprovedNodeTarget & { name: string }

export function NodeTestDialog(props: {
  target: TestTarget
  onClose: () => void
}) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const userId = useAuthStore((state) => state.auth.user?.id)
  const queryClient = useQueryClient()
  const active = useRef<AbortController | null>(null)
  useEffect(() => () => active.current?.abort(), [])

  const test = useMutation({
    mutationFn: () => {
      const controller = new AbortController()
      active.current = controller
      const target = props.target
      return testApprovedNode(
        {
          source_id: target.source_id,
          node_id: target.node_id,
          version_id: target.version_id,
        },
        controller.signal
      )
    },
    onSettled: () => {
      active.current = null
      return queryClient.invalidateQueries({
        queryKey: ['account-egress', userId, 'engines'],
      })
    },
    networkMode: 'always',
    meta: { errorToast: false },
    retry: false,
    gcTime: 0,
  })
  const close = () => {
    active.current?.abort()
    props.onClose()
  }
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) close()
      }}
      title={t('Verify node connection')}
      description={t(
        'Verify {{name}} using its approved version. The current account binding stays in place.',
        { name: props.target.name }
      )}
      footer={
        <>
          <Button variant='outline' onClick={close}>
            {test.isPending ? t('Cancel') : t('Close')}
          </Button>
          <Button disabled={test.isPending} onClick={() => test.mutate()}>
            {t('Verify connection')}
          </Button>
        </>
      }
    >
      <div className='flex flex-col gap-3'>
        {test.isPending && (
          <LoadingState
            message={t('Verifying the approved node connection…')}
          />
        )}
        {test.isError && isEgressCapacityError(test.error) && (
          <EnginePanel occupantsOnly />
        )}
        {test.isError && !isEgressCapacityError(test.error) && (
          <ErrorState
            title={t('Node connection verification failed')}
            description={
              getServerErrorStatus(test.error) === 429
                ? t('Too many requests')
                : getServerErrorMessage(test.error)
            }
          />
        )}
        {test.data && (
          <div role='status' className='space-y-1 text-sm'>
            <p className='font-medium'>
              {test.data.success
                ? t('Node connection verified')
                : t('Node connection failed')}
            </p>
            <p>
              {t('Latency (ms)')}: {formatNumber(test.data.latency_ms, locale)}
            </p>
            {test.data.exit_ipv4 && (
              <p>
                IPv4: <code>{test.data.exit_ipv4}</code>
              </p>
            )}
            {test.data.exit_ipv6 && (
              <p>
                IPv6: <code>{test.data.exit_ipv6}</code>
              </p>
            )}
            {!test.data.exit_ipv4 &&
              !test.data.exit_ipv6 &&
              test.data.exit_ip && (
                <p>
                  {t('Observed exit IP')}: <code>{test.data.exit_ip}</code>
                </p>
              )}
          </div>
        )}
      </div>
    </Dialog>
  )
}
