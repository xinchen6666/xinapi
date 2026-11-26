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
import { api } from '@/lib/api'
import {
  getServerErrorSources,
  getServerErrorStatus,
  requireServerSuccess,
} from '@/lib/server-error-message'
import { useAuthStore } from '@/stores/auth-store'

import type { Account, ListPage } from './types'

export type ApprovedNodeTarget = {
  source_id: string
  node_id: string
  version_id: string
}

export type RouteNode = ApprovedNodeTarget & {
  node_name: string
  protocol: string
}

export type ConnectionObservation = {
  success: boolean
  latency_ms: number
  exit_ip?: string | null
  exit_ipv4?: string | null
  exit_ipv6?: string | null
}

export type EngineEntry = RouteNode & {
  entry_id: string
  owner: { kind: 'account' | 'reservation'; id: string } | null
  state: string
  occupies_slot: boolean
  stoppable: boolean
  reclaimable: boolean
  expires_at?: string | null
  cold_start_ms?: number | null
  rss_bytes?: number | null
  cpu_seconds?: number | null
}

export type EngineInventory = {
  items: EngineEntry[]
  capacity: { limit: number; used: number; available: number }
}

export type ManagedRoute = {
  account_id: string
  proxy_id?: string | null
  proxy_enabled?: boolean | null
  account_proxy_id?: string | null
  generation: number
  current?: RouteNode | null
  observation?: ConnectionObservation | null
  applying_operation_id?: string | null
  applied_operation_id?: string | null
}

export type RouteReceipt = {
  operation: {
    operation_id: string
    phase: 'preparing' | 'ready' | 'applying' | 'applied' | 'failed'
    candidate: RouteNode
    observation?: ConnectionObservation | null
    error_code?: string | null
  }
  route?: ManagedRoute | null
  engine?: EngineEntry | null
  requires_reconciliation: boolean
}

export type AccountGroup = {
  id: string
  name: string
  color?: string | null
  enabled: boolean
}

export type AccountDetail = Account & {
  groups: AccountGroup[]
  capabilities: { quota: boolean; quotaRefresh: boolean }
  capacity?: { usedSlots: number | null; totalSlots?: number | null } | null
  quota?: {
    refreshedAt?: string | null
    limitReached?: boolean
    rateLimitedUntil?: string | null
    pausedWindows?: string[]
    credits?: {
      hasCredits: boolean
      unlimited: boolean
      balance?: string | null
    } | null
    windows?: Array<{
      key: string
      group: string
      role?: string | null
      usedPercent?: number | null
      limitReached?: boolean
      stale?: boolean
      labelDisplay?: string | null
      resetAtDisplay?: string | null
    }>
  } | null
}

type Result<T> = { success: boolean; data: T }
const prefix = '/api/account-egress'

async function get<T>(
  path: string,
  signal: AbortSignal,
  params?: Record<string, string | number>
): Promise<T> {
  const { data } = await api.get<Result<T>>(prefix + path, {
    params,
    signal,
    disableDuplicate: true,
    skipBusinessError: true,
  })
  return requireServerSuccess(data).data
}

async function post<T>(
  path: string,
  payload: object,
  signal?: AbortSignal
): Promise<T> {
  const { data } = await api.post<Result<T>>(prefix + path, payload, {
    signal,
    timeout: 80_000,
    singleUseAuthorization: true,
    expectedDashboardSession: useAuthStore.getState().auth.session?.sid ?? '',
    skipBusinessError: true,
  })
  return requireServerSuccess(data).data
}

export const getEngines = (signal: AbortSignal): Promise<EngineInventory> =>
  get('/engines', signal)
export const stopEngine = (entryId: string): Promise<{ engine: EngineEntry }> =>
  post('/engines/stop', { entry_id: entryId })
export const reclaimReservation = (
  operationId: string
): Promise<{ operation_id: string; status: string }> =>
  post('/engines/reclaim', { operation_id: operationId })
export const testApprovedNode = (
  target: ApprovedNodeTarget,
  signal: AbortSignal
): Promise<ConnectionObservation> => post('/engines/test-node', target, signal)

export const getManagedRoute = (
  accountId: string,
  signal: AbortSignal
): Promise<ManagedRoute> =>
  get('/managed-routes', signal, { account_id: accountId })
export const getRouteOperation = (
  operationId: string,
  signal: AbortSignal
): Promise<RouteReceipt> =>
  get('/managed-routes/operations', signal, { operation_id: operationId })
export const prepareManagedRoute = (input: {
  operation_id: string
  account_id: string
  expected_generation: number
  source_id: string
  node_id: string
}): Promise<RouteReceipt> => post('/managed-routes/prepare', input)
export const operateManagedRoute = (
  action: 'approve' | 'abort' | 'reconcile',
  operationId: string
): Promise<RouteReceipt> =>
  post(`/managed-routes/${action}`, { operation_id: operationId })

export const getAccountDetail = (
  accountId: string,
  signal: AbortSignal
): Promise<{ account: AccountDetail }> =>
  get('/accounts/detail', signal, { account_id: accountId })
export const getAccountGroups = (
  page: number,
  signal: AbortSignal
): Promise<ListPage<AccountGroup>> =>
  get('/account-groups/options', signal, { page, pageSize: 50 })
export const saveAccountSettings = (input: {
  account_id: string
  enabled?: boolean
  group_ids?: string[]
  expected_group_ids?: string[]
}): Promise<{ account: AccountDetail }> => post('/accounts/settings', input)
export const refreshAccountQuota = (
  accountId: string
): Promise<{ account: AccountDetail }> =>
  post('/accounts/quota/refresh', { account_id: accountId })

export function isEgressCapacityError(error: unknown): boolean {
  return (
    getServerErrorStatus(error) === 429 &&
    getServerErrorSources(error).some(
      (source) =>
        source.code === 'engine_capacity_reached' ||
        source.code === 'managed_route_capacity' ||
        source.code === 'account_egress_capacity'
    )
  )
}
