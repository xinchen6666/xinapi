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

import type { AccountOAuthProvider } from '../oauth-types'

export const accountOAuthStartSchema = z.object({
  provider: z.enum(['openai', 'xai']),
  name: z
    .string()
    .trim()
    .min(1)
    .refine((value) => [...value].length <= 100),
  source_id: z.union([z.literal(''), z.uuid()]),
  node_id: z.union([z.literal(''), z.uuid()]),
})
export type AccountOAuthStartValues = z.infer<typeof accountOAuthStartSchema>

export const accountOAuthCallbackSchema = z.object({
  callback_url: z
    .string()
    .trim()
    .min(1)
    .max(8192)
    .refine((value) => {
      try {
        const url = new URL(value)
        return (
          ['http:', 'https:'].includes(url.protocol) &&
          !url.username &&
          !url.password
        )
      } catch {
        return false
      }
    }),
})
export type AccountOAuthCallbackValues = z.infer<
  typeof accountOAuthCallbackSchema
>

export function accountAuthorizationLink(
  value: string | undefined,
  provider: AccountOAuthProvider
): string | null {
  if (!value) return null
  try {
    const url = new URL(value)
    if (
      url.protocol !== 'https:' ||
      url.port ||
      url.username ||
      url.password ||
      url.hash
    ) {
      return null
    }
    const hostname = provider === 'openai' ? 'auth.openai.com' : 'auth.x.ai'
    if (url.hostname === hostname) return url.href
    if (
      provider !== 'openai' ||
      url.hostname !== 'chatgpt.com' ||
      url.pathname !== '/codex/desktop-auth'
    ) {
      return null
    }
    // Desktop官方入口包装授权地址，内外两层分别限制到对应服务商
    const authorizeUrls = url.searchParams.getAll('authorize_url')
    if (authorizeUrls.length !== 1) return null
    const authorization = new URL(authorizeUrls[0])
    if (
      authorization.protocol !== 'https:' ||
      authorization.hostname !== 'auth.openai.com' ||
      authorization.pathname !== '/oauth/authorize' ||
      authorization.port ||
      authorization.username ||
      authorization.password ||
      authorization.hash
    ) {
      return null
    }
    return url.href
  } catch {
    return null
  }
}
