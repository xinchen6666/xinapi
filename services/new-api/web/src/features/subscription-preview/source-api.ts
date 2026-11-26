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
import { requireServerSuccess } from '@/lib/server-error-message'

import type {
  SourceApproval,
  SourceDetail,
  SourceSummary,
} from './source-types'

type Response<T> = { success: boolean; message: string; data: T }
const sourcePath = '/api/account-egress/subscription-sources'

export async function listSources(
  signal?: AbortSignal
): Promise<SourceSummary[]> {
  const response = await api.get<Response<{ items: SourceSummary[] }>>(
    sourcePath,
    { signal, disableDuplicate: true, skipBusinessError: true }
  )
  return requireServerSuccess(response.data).data.items ?? []
}
export async function sourceDetail(
  sourceId: string,
  signal?: AbortSignal
): Promise<SourceDetail> {
  const response = await api.get<Response<SourceDetail>>(
    `${sourcePath}/detail`,
    {
      params: { source_id: sourceId },
      signal,
      disableDuplicate: true,
      skipBusinessError: true,
    }
  )
  return requireServerSuccess(response.data).data
}
export async function saveSource(input: {
  name: string
  source_url?: string
  source_id?: string
  expected_revision?: number
}): Promise<SourceDetail> {
  const operation = input.source_id ? 'update' : 'create'
  const response = await api.post<Response<SourceDetail>>(
    `${sourcePath}/${operation}`,
    input,
    { singleUseAuthorization: true, skipBusinessError: true }
  )
  return requireServerSuccess(response.data).data
}
export async function refreshSource(input: {
  source_id: string
  expected_revision: number
}): Promise<SourceDetail> {
  const response = await api.post<Response<SourceDetail>>(
    `${sourcePath}/refresh`,
    input,
    { singleUseAuthorization: true, skipBusinessError: true }
  )
  return requireServerSuccess(response.data).data
}
export async function approveSource(
  input: SourceApproval
): Promise<SourceDetail> {
  const response = await api.post<Response<SourceDetail>>(
    `${sourcePath}/approve`,
    input,
    { singleUseAuthorization: true, skipBusinessError: true }
  )
  return requireServerSuccess(response.data).data
}
