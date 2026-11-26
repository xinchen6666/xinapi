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
  getServerErrorMessage,
  getServerErrorSources,
  getServerErrorStatus,
  requireServerSuccess,
  safeServerErrorMessage,
} from '@/lib/server-error-message'

import type {
  AccountOAuthReceipt,
  AccountOAuthStart,
  AccountOAuthStarted,
} from './oauth-types'

type Response<T> = { success: boolean; data: T }
const operationsPath = '/api/account-egress/oauth/operations'

// 授权错误仅保留安全提示与状态，原请求体随调用结束释放
export class AccountOAuthError extends Error {
  readonly [safeServerErrorMessage] = true
  readonly status: number | undefined
  readonly accessDenied: boolean

  constructor(error: unknown) {
    super(
      getServerErrorMessage(
        error,
        'Account authorization service is unavailable'
      )
    )
    this.status = getServerErrorStatus(error)
    this.accessDenied =
      this.status === 403 &&
      !getServerErrorSources(error).some(
        (source) =>
          source.code === 'AUTH_ORIGIN_FORBIDDEN' ||
          source.code === 'AUTH_SESSION_REQUIRED'
      )
  }
}

export async function listAccountOAuthOperations(
  signal: AbortSignal
): Promise<AccountOAuthReceipt[]> {
  const response = await api.get<Response<{ items: AccountOAuthReceipt[] }>>(
    operationsPath,
    { signal, disableDuplicate: true, skipBusinessError: true }
  )
  return requireServerSuccess(response.data).data.items
}

export async function getAccountOAuthReceipt(
  operationId: string,
  signal?: AbortSignal
): Promise<AccountOAuthReceipt> {
  const response = await api.get<Response<AccountOAuthReceipt>>(
    `${operationsPath}/${encodeURIComponent(operationId)}`,
    { signal, disableDuplicate: true, skipBusinessError: true }
  )
  return requireServerSuccess(response.data).data
}

export async function startAccountOAuth(
  input: AccountOAuthStart
): Promise<AccountOAuthStarted> {
  try {
    const response = await api.post<Response<AccountOAuthStarted>>(
      operationsPath,
      input,
      { singleUseAuthorization: true, skipBusinessError: true }
    )
    return requireServerSuccess(response.data).data
  } catch (error) {
    throw new AccountOAuthError(error)
  }
}

export async function completeAccountOAuth(
  operationId: string,
  callbackUrl: string
): Promise<AccountOAuthReceipt> {
  try {
    const response = await api.post<Response<AccountOAuthReceipt>>(
      `${operationsPath}/${encodeURIComponent(operationId)}/complete`,
      { callback_url: callbackUrl },
      { singleUseAuthorization: true, skipBusinessError: true }
    )
    return requireServerSuccess(response.data).data
  } catch (error) {
    throw new AccountOAuthError(error)
  }
}

export async function cancelAccountOAuth(
  operationId: string
): Promise<AccountOAuthReceipt> {
  try {
    const response = await api.post<Response<AccountOAuthReceipt>>(
      `${operationsPath}/${encodeURIComponent(operationId)}/cancel`,
      {},
      { singleUseAuthorization: true, skipBusinessError: true }
    )
    return requireServerSuccess(response.data).data
  } catch (error) {
    throw new AccountOAuthError(error)
  }
}
