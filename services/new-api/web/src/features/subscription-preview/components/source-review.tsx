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
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ConfirmDialog } from '@/components/confirm-dialog'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { FieldError } from '@/components/ui/field'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber } from '@/lib/format'

import {
  buildDiagnosticRows,
  diagnosticMessage,
} from '../lib/subscription-preview'
import type {
  SourceApproval,
  SourceDetail,
  SourceSelection,
} from '../source-types'
import { DiagnosticsTable } from './diagnostics-table'
import { ReviewCandidates, type CandidateDecisions } from './review-candidates'

type Props = {
  detail: SourceDetail
  canWrite: boolean
  pending: boolean
  onApprove: (input: SourceApproval) => void
}

export function SourceReviewPanel(props: Props) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const review = props.detail.review
  const [decisions, setDecisions] = useState<CandidateDecisions>(() => {
    const result: CandidateDecisions = {}
    const suggestions = new Map<string, number>()
    for (const candidate of review?.candidates ?? []) {
      if (candidate.suggested_node_id) {
        suggestions.set(
          candidate.suggested_node_id,
          (suggestions.get(candidate.suggested_node_id) ?? 0) + 1
        )
      }
    }
    for (const candidate of review?.candidates ?? []) {
      let target = ''
      if (candidate.change === 'added') target = 'new'
      if (
        (candidate.change === 'unchanged' || candidate.change === 'renamed') &&
        candidate.suggested_node_id &&
        suggestions.get(candidate.suggested_node_id) === 1
      ) {
        target = candidate.suggested_node_id
      }
      result[candidate.index] = { selected: candidate.parse_valid, target }
    }
    return result
  })
  const [acceptPartial, setAcceptPartial] = useState(false)
  const [confirmRemovals, setConfirmRemovals] = useState(false)
  const [confirmOpen, setConfirmOpen] = useState(false)
  const selected = useMemo(
    () =>
      (review?.candidates ?? []).filter(
        (candidate) =>
          candidate.parse_valid && decisions[candidate.index]?.selected
      ),
    [review, decisions]
  )
  const skipped = (review?.candidates ?? []).filter(
    (candidate) => !selected.includes(candidate)
  )
  const mapping: SourceSelection[] = selected.map((candidate) => ({
    candidate_index: candidate.index,
    node_id:
      decisions[candidate.index]?.target === 'new'
        ? null
        : decisions[candidate.index]?.target || null,
  }))
  const mappedIds = mapping.flatMap((item) =>
    item.node_id ? [item.node_id] : []
  )
  const duplicate = new Set(mappedIds).size !== mappedIds.length
  const unresolved = selected.some(
    (candidate) => !decisions[candidate.index]?.target
  )
  const removals = props.detail.nodes.filter(
    (node) => node.available && !mappedIds.includes(node.id)
  )
  if (!review) return null
  const needsPartial =
    !review.complete ||
    selected.length !==
      review.candidates.filter((candidate) => candidate.parse_valid).length
  const ready =
    !duplicate &&
    !unresolved &&
    (!needsPartial || acceptPartial) &&
    (!review.complete || removals.length === 0 || confirmRemovals)
  return (
    <section className='space-y-4 rounded-lg border p-3'>
      <div>
        <h3 className='font-semibold'>{t('Pending source review')}</h3>
        <p className='text-muted-foreground text-sm'>
          {review.complete
            ? t(
                'Complete review: prior nodes outside the selection are marked unavailable after confirmation.'
              )
            : t(
                'Partial review: approved prior nodes are retained. Confirm partial acceptance to continue.'
              )}
        </p>
      </div>
      <ReviewCandidates
        candidates={review.candidates}
        nodes={props.detail.nodes}
        decisions={decisions}
        canWrite={props.canWrite && !props.pending}
        onChange={(index, value) =>
          setDecisions((current) => ({ ...current, [index]: value }))
        }
      />
      <DiagnosticsTable diagnostics={review.diagnostics} />
      {unresolved ? (
        <FieldError>
          {t('Choose an existing or new identity for each selected candidate.')}
        </FieldError>
      ) : null}
      {duplicate ? (
        <FieldError>
          {t('Each existing node identity can be selected once.')}
        </FieldError>
      ) : null}
      {props.canWrite ? (
        <div className='space-y-3'>
          {needsPartial ? (
            <label className='flex items-start gap-2 text-sm'>
              <Checkbox
                checked={acceptPartial}
                onCheckedChange={(value) => setAcceptPartial(Boolean(value))}
                disabled={props.pending}
              />
              {t('Accept the selected candidate scope')}
            </label>
          ) : null}
          {review.complete && removals.length > 0 ? (
            <label className='flex items-start gap-2 text-sm'>
              <Checkbox
                checked={confirmRemovals}
                onCheckedChange={(value) => setConfirmRemovals(Boolean(value))}
                disabled={props.pending}
              />
              {t(
                'Confirm marking prior nodes outside the selected identities unavailable'
              )}
            </label>
          ) : null}
          <Button
            disabled={!ready || props.pending}
            onClick={() => setConfirmOpen(true)}
          >
            {t('Review approval')}
          </Button>
        </div>
      ) : null}
      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        title={t('Approve source version')}
        desc={t(
          'Approval saves the selected node identities and connection versions.'
        )}
        confirmText={t('Approve source version')}
        disabled={!ready || !props.canWrite}
        isLoading={props.pending}
        className='sm:max-w-2xl'
        handleConfirm={() => {
          props.onApprove({
            source_id: props.detail.source.id,
            expected_revision: props.detail.source.revision,
            review_id: review.id,
            selected: mapping,
            accept_partial: acceptPartial,
            confirm_removals: confirmRemovals,
          })
          setConfirmOpen(false)
        }}
      >
        <div className='max-h-[55dvh] space-y-4 overflow-y-auto text-sm'>
          <section>
            <h4 className='font-medium'>
              {t('Selected nodes: {{count}}', {
                count: formatNumber(selected.length, locale),
              })}
            </h4>
            <ul className='list-disc space-y-1 pl-5'>
              {selected.map((candidate) => (
                <li key={candidate.index} className='break-all'>
                  {candidate.name} · {candidate.protocol} ·{' '}
                  {decisions[candidate.index]?.target === 'new'
                    ? t('Create new node identity')
                    : decisions[candidate.index]?.target}
                </li>
              ))}
            </ul>
          </section>
          <section>
            <h4 className='font-medium'>
              {t('Skipped nodes: {{count}}', {
                count: formatNumber(skipped.length, locale),
              })}
            </h4>
            <ul className='list-disc pl-5'>
              {skipped.map((candidate) => (
                <li key={candidate.index} className='break-all'>
                  {candidate.name} · {candidate.protocol}
                </li>
              ))}
            </ul>
          </section>
          <section>
            <h4 className='font-medium'>{t('Missing prior nodes')}</h4>
            <ul className='list-disc pl-5'>
              {review.missing_node_ids.map((id) => (
                <li key={id} className='break-all'>
                  {props.detail.nodes.find((node) => node.id === id)?.name ??
                    id}{' '}
                  · {id}
                </li>
              ))}
            </ul>
          </section>
          <section>
            <h4 className='font-medium'>
              {review.complete
                ? t('Prior identities to mark unavailable')
                : t('Prior identities retained')}
            </h4>
            <ul className='list-disc pl-5'>
              {(review.complete ? removals : props.detail.nodes).map((node) => (
                <li key={node.id} className='break-all'>
                  {node.name} · {node.id}
                </li>
              ))}
            </ul>
          </section>
          <section>
            <h4 className='font-medium'>
              {t('Diagnostics to acknowledge: {{count}}', {
                count: formatNumber(review.diagnostics.length, locale),
              })}
            </h4>
            <ul className='space-y-2'>
              {buildDiagnosticRows(review.diagnostics).map((diagnostic) => (
                <li key={diagnostic.id} className='break-all'>
                  {diagnostic.subject} · {diagnostic.code}
                  <p>{diagnosticMessage(diagnostic, t)}</p>
                </li>
              ))}
            </ul>
          </section>
        </div>
      </ConfirmDialog>
    </section>
  )
}
