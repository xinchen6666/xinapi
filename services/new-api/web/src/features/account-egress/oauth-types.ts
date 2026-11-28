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
export type AccountOAuthProvider = 'openai' | 'xai'

export type AccountOAuthStatus =
  | 'pending'
  | 'completing'
  | 'completed'
  | 'failed'
  | 'unknown'
  | 'cancelled'
  | 'expired'

export type AccountOAuthReceipt = {
  name?: string
  operation_id: string
  status: AccountOAuthStatus
  provider: AccountOAuthProvider
  account_id?: string | null
  expires_at: string
  source_id?: string | null
  node_id?: string | null
  version_id?: string | null
  proxy_id?: string | null
  route_generation: number
  reason?: string | null
  authorization_url?: string
}

export type AccountOAuthStarted = AccountOAuthReceipt & {
  authorization_url?: string
}

export type AccountOAuthStart = {
  provider: AccountOAuthProvider
  name: string
  account_id?: string
  source_id?: string
  node_id?: string
}

export type AccountOAuthTarget = {
  provider: AccountOAuthProvider
  account_id: string
  name: string
}
