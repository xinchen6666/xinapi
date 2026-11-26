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
import {
  useIsMutating,
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryResult,
} from '@tanstack/react-query'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { v4 as newOperationId } from 'uuid'

import { ConfirmDialog } from '@/components/confirm-dialog'
import { ErrorState } from '@/components/error-state'
import { LoadingState } from '@/components/loading-state'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber } from '@/lib/format'
import {
  getServerErrorMessage,
  getServerErrorSources,
  getServerErrorStatus,
} from '@/lib/server-error-message'

import {
  getEngines,
  isEgressCapacityError,
  getRouteOperation,
  operateManagedRoute,
  prepareManagedRoute,
  type AccountDetail,
  type ManagedRoute,
  type RouteNode,
  type RouteReceipt,
} from './console-api'
import { EnginePanel } from './engine-panel'
import type { AccountOAuthTarget } from './oauth-types'
import { RouteNodeFields } from './route-node-fields'

type RouteCommand =
  | { kind: 'prepare'; id: string; generation: number; candidate: RouteNode }
  | { kind: 'approve' | 'abort' | 'reconcile' | 'resume'; id: string }
type ConfirmedCommand = Exclude<RouteCommand, { kind: 'prepare' }>
type RoutePanelProps = {
  account: AccountDetail
  userId: number
  sessionId: string
  route: UseQueryResult<ManagedRoute | null>
  mutationKey: readonly unknown[]
  canManage: boolean
  canAuthorize: boolean
  onReauthorize: (target: AccountOAuthTarget) => void
}

export function ManagedRoutePanel(props: RoutePanelProps) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const queryClient = useQueryClient()
  const [selected, setSelected] = useState<RouteNode | null>(null)
  const [prepared, setPrepared] = useState<Extract<
    RouteCommand,
    { kind: 'prepare' }
  > | null>(null)
  const [recentId, setRecentId] = useState('')
  const [unknownId, setUnknownId] = useState('')
  const [capacitySince, setCapacitySince] = useState<number | null>(null)
  const [confirm, setConfirm] = useState<ConfirmedCommand | null>(null)
  const [idError, setIdError] = useState(false)
  const accountId = props.account.id
  const route = props.route.data
  const activeId = route?.applying_operation_id
  const appliedId = route?.applied_operation_id
  const operationId = unknownId || activeId || recentId || appliedId || ''
  const operationKey = (id: string) =>
    [
      'account-egress',
      props.userId,
      'route-operation',
      props.sessionId,
      accountId,
      id,
    ] as const
  const operationQuery = useQuery({
    queryKey: operationKey(operationId),
    queryFn: ({ signal }) => getRouteOperation(operationId, signal),
    enabled: !!operationId && !unknownId,
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const capacity = useQuery({
    queryKey: ['account-egress', props.userId, 'engines', props.sessionId],
    queryFn: ({ signal }) => getEngines(signal),
    enabled: capacitySince !== null && props.canManage,
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const change = useMutation({
    mutationKey: props.mutationKey,
    mutationFn: (command: RouteCommand) => {
      if (command.kind === 'prepare') {
        return prepareManagedRoute({
          operation_id: command.id,
          account_id: accountId,
          expected_generation: command.generation,
          source_id: command.candidate.source_id,
          node_id: command.candidate.node_id,
        })
      }
      return operateManagedRoute(
        command.kind === 'resume' ? 'reconcile' : command.kind,
        command.id
      )
    },
    onSuccess: (receipt) => {
      setUnknownId('')
      setRecentId(receipt.operation.operation_id)
      queryClient.setQueryData(
        operationKey(receipt.operation.operation_id),
        receipt
      )
    },
    onError: (error, command) => {
      const status = getServerErrorStatus(error)
      if (command.kind === 'prepare' && isEgressCapacityError(error)) {
        setUnknownId('')
        setCapacitySince(Date.now())
      } else if (
        status == null ||
        getServerErrorSources(error).some(
          (source) => source.code === 'account_egress_result_unknown'
        )
      ) {
        setUnknownId(command.id)
      }
    },
    onSettled: () =>
      queryClient.invalidateQueries({
        queryKey: ['account-egress', props.userId],
      }),
    networkMode: 'always',
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const busy =
    useIsMutating({ mutationKey: props.mutationKey, exact: true }) > 0
  const receipt = operationQuery.data
  const operation = receipt?.operation
  const candidate = operation?.candidate
  const state = operation?.phase
  const pending =
    state === 'preparing' || state === 'ready' || state === 'applying'
  const generation = route?.generation ?? 0
  const bound = generation > 0 && !!route?.current
  const ownedProxy =
    !!route?.proxy_id &&
    route.proxy_id === route.account_proxy_id &&
    route.proxy_id === props.account.outboundProxyId
  const provider = props.account.provider
  const canReauthorize =
    props.canAuthorize &&
    props.account.enabled &&
    bound &&
    ownedProxy &&
    route?.proxy_enabled === true &&
    !activeId &&
    !unknownId &&
    (provider === 'openai' || provider === 'xai')
  const ownPrepare = prepared?.id === operationId ? prepared : null
  const versionChanged =
    !!ownPrepare &&
    !!candidate &&
    (ownPrepare.candidate.source_id !== candidate.source_id ||
      ownPrepare.candidate.node_id !== candidate.node_id ||
      ownPrepare.candidate.version_id !== candidate.version_id)
  const generationChanged = !!ownPrepare && ownPrepare.generation !== generation
  const availableAfterRelease =
    capacitySince === null ||
    (capacity.isSuccess &&
      capacity.dataUpdatedAt >= capacitySince &&
      capacity.data.capacity.available > 0)
  const freshRoute = props.route.isSuccess && !props.route.isFetching
  const canWrite =
    props.canManage &&
    !busy &&
    !unknownId &&
    freshRoute &&
    !operationQuery.isFetching &&
    !operationQuery.isError
  const canApprove =
    canWrite &&
    state === 'ready' &&
    !activeId &&
    !versionChanged &&
    !generationChanged
  const canResume =
    canWrite &&
    !activeId &&
    !pending &&
    !!appliedId &&
    bound &&
    ownedProxy &&
    route?.proxy_enabled === true &&
    receipt?.requires_reconciliation &&
    receipt.engine?.state !== 'starting'
  const confirmValid = confirm
    ? (() => {
        switch (confirm.kind) {
          case 'approve':
            return canApprove && confirm.id === operationId
          case 'abort':
            return canWrite && pending && confirm.id === operationId
          case 'reconcile':
            return (
              canWrite &&
              (state === 'preparing' || state === 'applying') &&
              confirm.id === (activeId ?? operationId)
            )
          case 'resume':
            return canResume && confirm.id === appliedId
        }
      })()
    : false

  const readOperation = async () => {
    const result = await operationQuery.refetch()
    if (result.isSuccess) {
      setRecentId(result.data.operation.operation_id)
      setUnknownId('')
      change.reset()
    }
    await props.route.refetch()
  }
  const canContinueSavedPrepare =
    !!prepared &&
    prepared.id === unknownId &&
    operationQuery.isError &&
    getServerErrorStatus(operationQuery.error) === 404 &&
    getServerErrorSources(operationQuery.error).some(
      (source) => source.code === 'managed_route_missing'
    ) &&
    props.canManage &&
    !busy &&
    !operationQuery.isFetching &&
    freshRoute &&
    generation === prepared.generation &&
    !activeId

  const prepare = (node: RouteNode) => {
    if (!canWrite || !availableAfterRelease || activeId || pending) return
    let id: string
    try {
      id = newOperationId()
    } catch {
      setIdError(true)
      return
    }
    const command = {
      kind: 'prepare',
      id,
      generation,
      candidate: node,
    } as const
    setPrepared(command)
    setIdError(false)
    setCapacitySince(null)
    change.mutate(command)
  }

  return (
    <section className='min-w-0 space-y-3 rounded-lg border p-3'>
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <h3 className='font-semibold'>{t('Managed route')}</h3>
        {props.route.isSuccess && (
          <Button
            size='sm'
            variant='outline'
            disabled={
              busy || props.route.isFetching || operationQuery.isFetching
            }
            onClick={() => {
              if (operationId) void readOperation()
              else void props.route.refetch()
            }}
          >
            {t('Refresh')}
          </Button>
        )}
      </div>
      {props.route.isPending && <LoadingState />}
      {props.route.isError && (
        <ErrorState
          className='min-h-24'
          title={t('Managed route is unavailable')}
          onRetry={() => void props.route.refetch()}
        />
      )}
      {props.route.isSuccess && (
        <>
          <p className='text-sm'>
            {bound && route?.current
              ? t('Bound to {{node}}', { node: route.current.node_name })
              : t('No managed route is bound.')}
            {' · '}
            {t('Route generation')}: {formatNumber(generation, locale)}
          </p>
          {bound && route?.current && (
            <RouteNodeSummary
              node={route.current}
              observation={route.observation}
            />
          )}
          {canReauthorize && (provider === 'openai' || provider === 'xai') && (
            <Button
              variant='outline'
              size='sm'
              disabled={busy || !freshRoute}
              onClick={() =>
                props.onReauthorize({
                  account_id: props.account.id,
                  name: props.account.name,
                  provider,
                })
              }
            >
              {t('Reauthorize account')}
            </Button>
          )}
        </>
      )}
      {props.route.isSuccess &&
        receipt &&
        operation &&
        !unknownId &&
        !operationQuery.isError && (
          <div className='space-y-2 rounded-md border p-3 text-sm'>
            <div className='flex flex-wrap items-center gap-2'>
              <Badge variant='outline'>
                <RoutePhase receipt={receipt} />
              </Badge>
              <code className='text-muted-foreground break-all'>
                {operation.operation_id}
              </code>
            </div>
            <p className='font-medium'>
              {t('Candidate node')}: {operation.candidate.node_name}
            </p>
            <RouteNodeSummary
              node={operation.candidate}
              observation={operation.observation}
            />
            {versionChanged && (
              <p role='alert' className='text-destructive'>
                {t(
                  'The prepared version differs from the selected version. Cancel it and prepare the approved node again.'
                )}
              </p>
            )}
            {generationChanged && state === 'ready' && (
              <p role='alert' className='text-destructive'>
                {t(
                  'The current route changed. Prepare a new candidate for the latest route generation.'
                )}
              </p>
            )}
            {props.canManage && (
              <div className='flex flex-wrap gap-2'>
                {state === 'ready' && (
                  <Button
                    size='sm'
                    disabled={!canApprove}
                    onClick={() =>
                      setConfirm({
                        kind: 'approve',
                        id: operation.operation_id,
                      })
                    }
                  >
                    {t('Approve route')}
                  </Button>
                )}
                {pending && (
                  <Button
                    size='sm'
                    variant='outline'
                    disabled={!canWrite}
                    onClick={() =>
                      setConfirm({ kind: 'abort', id: operation.operation_id })
                    }
                  >
                    {t('Cancel route operation')}
                  </Button>
                )}
                {(state === 'applying' || state === 'preparing') && (
                  <Button
                    size='sm'
                    variant='outline'
                    disabled={!canWrite}
                    onClick={() =>
                      setConfirm({
                        kind: 'reconcile',
                        id: activeId ?? operation.operation_id,
                      })
                    }
                  >
                    {t('Reconcile route')}
                  </Button>
                )}
              </div>
            )}
          </div>
        )}
      {canResume && appliedId && (
        <Button
          size='sm'
          variant='outline'
          onClick={() => setConfirm({ kind: 'resume', id: appliedId })}
        >
          {t('Resume managed route')}
        </Button>
      )}
      {unknownId && (
        <div className='space-y-2 rounded-md border p-3 text-sm'>
          <p role='alert'>
            {t(
              'The route result is unknown. Read the saved operation before continuing.'
            )}
          </p>
          <code className='block break-all'>{unknownId}</code>
          <Button
            size='sm'
            variant='outline'
            disabled={operationQuery.isFetching || busy}
            onClick={() => void readOperation()}
          >
            {t('Check operation status')}
          </Button>
          {canContinueSavedPrepare && prepared && (
            <>
              <RouteNodeSummary node={prepared.candidate} />
              <Button
                size='sm'
                variant='outline'
                onClick={() => {
                  if (canContinueSavedPrepare) change.mutate(prepared)
                }}
              >
                {t('Continue preparing route')}
              </Button>
            </>
          )}
        </div>
      )}
      {operationQuery.isError && (
        <ErrorState
          className='min-h-24'
          title={t('Route operation is unavailable')}
          description={getServerErrorMessage(operationQuery.error)}
          onRetry={() => void readOperation()}
        />
      )}
      {props.canManage &&
        props.route.isSuccess &&
        !activeId &&
        !pending &&
        !unknownId &&
        !operationQuery.isError && (
          <>
            {capacitySince !== null && (
              <div className='space-y-2 rounded-md border p-3'>
                <p role='alert' className='text-sm'>
                  {t(
                    'Free an engine slot, then choose Continue preparing route.'
                  )}
                </p>
                <EnginePanel occupantsOnly />
              </div>
            )}
            <RouteNodeFields
              userId={props.userId}
              sessionId={props.sessionId}
              selected={selected}
              onSelectionChange={setSelected}
              onPrepare={prepare}
              busy={busy}
              mayPrepare={canWrite && availableAfterRelease}
              continuation={capacitySince !== null}
            />
          </>
        )}
      {idError && (
        <p className='text-destructive text-sm' role='alert'>
          {t(
            'An operation ID could not be created. Open the console in a secure browser context and try again.'
          )}
        </p>
      )}
      {change.isError && !unknownId && capacitySince === null && (
        <p role='alert' className='text-destructive text-sm'>
          {getServerErrorStatus(change.error) === 429 &&
          !isEgressCapacityError(change.error)
            ? t('Too many requests')
            : getServerErrorMessage(change.error)}
        </p>
      )}
      {confirm && props.canManage && (
        <RouteActionConfirmation
          command={confirm}
          busy={busy}
          disabled={!confirmValid}
          onClose={() => setConfirm(null)}
          onSubmit={() => {
            if (!confirmValid) return
            setConfirm(null)
            change.mutate(confirm)
          }}
        />
      )}
    </section>
  )
}

function RouteNodeSummary(props: {
  node: RouteNode
  observation?: RouteReceipt['operation']['observation']
}) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const observation = props.observation
  return (
    <div className='text-muted-foreground space-y-1 text-sm'>
      <p className='break-words'>
        {t('Approved version')}:{' '}
        <code className='break-all'>{props.node.version_id}</code>
      </p>
      <p>
        {t('Protocol')}: {props.node.protocol}
      </p>
      {observation && (
        <div className='flex flex-wrap gap-x-3 gap-y-1'>
          <span>
            {t('Latency (ms)')}: {formatNumber(observation.latency_ms, locale)}
          </span>
          {observation.exit_ipv4 && (
            <span>
              {t('IPv4')}: <code>{observation.exit_ipv4}</code>
            </span>
          )}
          {observation.exit_ipv6 && (
            <span>
              {t('IPv6')}: <code>{observation.exit_ipv6}</code>
            </span>
          )}
          {!observation.exit_ipv4 &&
            !observation.exit_ipv6 &&
            observation.exit_ip && (
              <span>
                {t('Observed exit IP')}: <code>{observation.exit_ip}</code>
              </span>
            )}
        </div>
      )}
    </div>
  )
}

function RoutePhase(props: { receipt: RouteReceipt }) {
  const { t } = useTranslation()
  switch (props.receipt.operation.phase) {
    case 'preparing':
      return t('Preparing')
    case 'ready':
      return t('Ready for approval')
    case 'applying':
      return t('Applying')
    case 'applied':
      return t('Applied')
    case 'failed': {
      if (props.receipt.operation.error_code === 'route_operation_cancelled') {
        return t('Cancelled')
      }
      if (props.receipt.operation.error_code === 'route_restore_required') {
        return t('Route reconciliation required')
      }
      return t('Failed')
    }
  }
}

function RouteActionConfirmation(props: {
  command: ConfirmedCommand
  busy: boolean
  disabled: boolean
  onClose: () => void
  onSubmit: () => void
}) {
  const { t } = useTranslation()
  const text = {
    approve: {
      title: t('Approve route'),
      desc: t(
        'Publishing this route ends connections using the previous route.'
      ),
    },
    abort: {
      title: t('Cancel route operation'),
      desc: t(
        'Cancel this candidate. If it was applying, the system attempts to resume the previous route.'
      ),
    },
    reconcile: {
      title: t('Reconcile route'),
      desc: t(
        'Use the saved operation to resolve the interrupted route change.'
      ),
    },
    resume: {
      title: t('Resume managed route'),
      desc: t(
        'Resume using the fixed version already published for this account.'
      ),
    },
  }[props.command.kind]
  return (
    <ConfirmDialog
      open
      onOpenChange={(open) => {
        if (!open && !props.busy) props.onClose()
      }}
      title={text.title}
      desc={text.desc}
      confirmText={text.title}
      destructive={
        props.command.kind === 'approve' || props.command.kind === 'abort'
      }
      disabled={props.disabled}
      isLoading={props.busy}
      handleConfirm={props.onSubmit}
    />
  )
}
