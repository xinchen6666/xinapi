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
import { isAxiosError } from 'axios'

import type {
  CandidateGroup,
  CandidateRow,
  DiagnosticRow,
  SubscriptionPreviewCandidate,
  SubscriptionPreviewDiagnostic,
} from '../types'

/** The server accepts links up to this length; the field mirrors the limit. */
export const SUBSCRIPTION_SOURCE_MAX_LENGTH = 2048

/**
 * Returns the trimmed absolute http(s) address, or null when the text still
 * needs an edit. The server validates the same rules; this check only saves a
 * round trip.
 */
export function normalizeSubscriptionSource(value: string): string | null {
  const trimmed = value.trim()
  if (trimmed === '' || trimmed.length > SUBSCRIPTION_SOURCE_MAX_LENGTH) {
    return null
  }

  let parsed: URL
  try {
    parsed = new URL(trimmed)
  } catch {
    return null
  }
  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') return null
  if (parsed.hostname === '') return null
  return trimmed
}

/** Rows keep the submitted order inside each protocol. */
export function buildCandidateRows(
  candidates: SubscriptionPreviewCandidate[]
): CandidateRow[] {
  return candidates.map((candidate, index) => ({
    id: `${candidate.protocol}:${index}`,
    name: candidate.name,
    protocol: candidate.protocol,
    supported: candidate.supported,
  }))
}

/** Groups rows by protocol, so the table reads as one block per protocol. */
export function groupCandidateRows(rows: CandidateRow[]): CandidateGroup[] {
  const groups = new Map<string, CandidateRow[]>()
  for (const row of rows) {
    const protocolRows = groups.get(row.protocol)
    if (protocolRows) {
      protocolRows.push(row)
    } else {
      groups.set(row.protocol, [row])
    }
  }

  return Array.from(groups, ([protocol, protocolRows]) => ({
    protocol,
    rows: protocolRows,
    accepted: protocolRows.filter((row) => row.supported).length,
  })).sort((left, right) => left.protocol.localeCompare(right.protocol))
}

export function buildDiagnosticRows(
  diagnostics: SubscriptionPreviewDiagnostic[]
): DiagnosticRow[] {
  return diagnostics.map((diagnostic, index) => ({
    ...diagnostic,
    id: `${diagnostic.code}:${index}`,
  }))
}

/**
 * The stable failure code the management API adds to a rejected preview, so an
 * administrator can report the exact reason next to the translated message.
 */
export function previewErrorCode(error: unknown): string | null {
  if (!isAxiosError(error)) return null
  const body: unknown = error.response?.data
  if (typeof body !== 'object' || body === null) return null
  const code = (body as { code?: unknown }).code
  return typeof code === 'string' && code !== '' ? code : null
}
