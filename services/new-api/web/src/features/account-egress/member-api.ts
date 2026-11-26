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
import { z } from 'zod'

import { api } from '@/lib/api'
import { requireServerSuccess } from '@/lib/server-error-message'

const memberAccessSchema = z.object({
  user_id: z.number().int().positive(),
  revision: z.number().int().nonnegative(),
  group_ids: z.array(z.string()),
  status: z.enum(['active', 'pending', 'unknown', 'revoked']),
  operation_id: z.string().nullish(),
})

export type MemberAccess = z.infer<typeof memberAccessSchema>

export type MemberAccessCommand =
  | { action: 'groups'; group_ids: string[]; expected_revision: number }
  | { action: 'reconcile'; operation_id: string; expected_revision: number }

type MemberAccessResult = { success: boolean; data: unknown }

function readMemberAccessResult(
  response: MemberAccessResult,
  userId: number
): MemberAccess {
  const result = memberAccessSchema.parse(requireServerSuccess(response).data)
  if (result.user_id !== userId) {
    throw new Error('Unexpected member access owner')
  }
  return result
}

export async function getMemberAccess(
  userId: number,
  sessionId: string,
  signal: AbortSignal
): Promise<MemberAccess> {
  const { data } = await api.get<MemberAccessResult>(
    `/api/account-egress/members/${userId}`,
    {
      signal,
      disableDuplicate: true,
      skipBusinessError: true,
      skipErrorHandler: true,
      singleUseAuthorization: true,
      expectedDashboardSession: sessionId,
    }
  )
  return readMemberAccessResult(data, userId)
}

export async function changeMemberAccess(
  userId: number,
  sessionId: string,
  command: MemberAccessCommand
): Promise<MemberAccess> {
  const { action, ...payload } = command
  const { data } = await api.post<MemberAccessResult>(
    `/api/account-egress/members/${userId}/${action}`,
    payload,
    {
      timeout: 80_000,
      skipBusinessError: true,
      skipErrorHandler: true,
      singleUseAuthorization: true,
      expectedDashboardSession: sessionId,
    }
  )
  return readMemberAccessResult(data, userId)
}
