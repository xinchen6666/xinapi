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
import type { SubscriptionPreviewDiagnostic } from './types'

export type SourceSummary = {
  id: string
  name: string
  revision: number
  approved_version: number
  status: string
  last_error_code?: string
  node_count: number
  has_pending_review: boolean
  updated_at: string
}
export type SourceNode = {
  id: string
  name: string
  protocol: string
  version_id: string
  available: boolean
  supported: boolean
}
export type SourceCandidate = {
  index: number
  name: string
  protocol: string
  parse_valid: boolean
  supported: boolean
  change:
    | 'unchanged'
    | 'renamed'
    | 'added'
    | 'connection_changed'
    | 'ambiguous'
    | 'invalid'
  suggested_node_id?: string
  previous_node_ids: string[]
}
export type SourceReview = {
  id: string
  source_id: string
  expected_revision: number
  complete: boolean
  candidates: SourceCandidate[]
  missing_node_ids: string[]
  diagnostics: SubscriptionPreviewDiagnostic[]
  created_at: string
}
export type SourceDetail = {
  source: SourceSummary
  nodes: SourceNode[]
  review?: SourceReview
}
export type SourceSelection = {
  candidate_index: number
  node_id: string | null
}
export type SourceApproval = {
  source_id: string
  expected_revision: number
  review_id: string
  selected: SourceSelection[]
  accept_partial: boolean
  confirm_removals: boolean
}
