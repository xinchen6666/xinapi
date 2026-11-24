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
import type { OnChangeFn, RowSelectionState } from '@tanstack/react-table'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber } from '@/lib/format'

import { buildCandidateRows } from '../lib/subscription-preview'
import type { CandidateRow, SubscriptionPreview } from '../types'
import { CandidatesTable } from './candidates-table'
import { DiagnosticsTable } from './diagnostics-table'
import { PreviewConfirmation } from './preview-confirmation'
import { PreviewSummary } from './preview-summary'

type PreviewResultProps = { preview: SubscriptionPreview }

export function PreviewResult(props: PreviewResultProps) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const rows = useMemo(
    () => buildCandidateRows(props.preview.candidates),
    [props.preview.candidates]
  )
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({})
  const [confirmationOpen, setConfirmationOpen] = useState(false)
  const [confirmedRows, setConfirmedRows] = useState<CandidateRow[] | null>(
    null
  )
  const selectedRows = rows.filter(
    (row) => row.parse_valid && rowSelection[row.id]
  )
  const skippedRows = rows.filter(
    (row) => !row.parse_valid || !rowSelection[row.id]
  )
  const requiresConfirmation = props.preview.diagnostics.length > 0
  const changeSelection: OnChangeFn<RowSelectionState> = (updater) => {
    setConfirmedRows(null)
    setRowSelection(updater)
  }

  return (
    <div className='flex flex-col gap-4'>
      <PreviewSummary preview={props.preview} />
      <p className='text-muted-foreground text-sm'>
        {t(
          'Parse validity describes connection metadata. Native HTTP/SOCKS support and Mihomo integration describe transport capability.'
        )}
      </p>
      {requiresConfirmation && (
        <Alert>
          <AlertTitle>{t('Review the partial preview')}</AlertTitle>
          <AlertDescription>
            {t(
              'Select parse-valid nodes and confirm the scope after reviewing skipped nodes and diagnostics.'
            )}
          </AlertDescription>
        </Alert>
      )}
      <section className='flex flex-col gap-2'>
        <h3 className='text-sm font-medium'>{t('Nodes')}</h3>
        <CandidatesTable
          rows={rows}
          enableSelection={requiresConfirmation}
          rowSelection={rowSelection}
          onRowSelectionChange={changeSelection}
        />
      </section>
      {requiresConfirmation && (
        <div className='flex flex-wrap items-center gap-3'>
          <Button
            disabled={selectedRows.length === 0}
            onClick={() => setConfirmationOpen(true)}
          >
            {t('Review selected scope')}
          </Button>
          <span className='text-muted-foreground text-sm'>
            {t('Selected nodes: {{count}}', {
              count: formatNumber(selectedRows.length, locale),
            })}
          </span>
        </div>
      )}
      {confirmedRows && (
        <Alert>
          <AlertTitle>
            {t('Confirmed for this preview: {{count}} nodes', {
              count: formatNumber(confirmedRows.length, locale),
            })}
          </AlertTitle>
          <AlertDescription>
            <p>
              {t(
                'Confirmation stays in this preview. Accounts and runtime configuration stay unchanged.'
              )}
            </p>
            <ul className='max-h-40 list-disc space-y-1 overflow-y-auto pl-5'>
              {confirmedRows.map((row) => (
                <li key={row.id} className='break-all'>
                  {row.name || '—'} · {row.protocol}
                </li>
              ))}
            </ul>
          </AlertDescription>
        </Alert>
      )}
      <section className='flex flex-col gap-2'>
        <h3 className='text-sm font-medium'>{t('Diagnostics')}</h3>
        <DiagnosticsTable diagnostics={props.preview.diagnostics} />
      </section>
      <PreviewConfirmation
        open={confirmationOpen}
        onOpenChange={setConfirmationOpen}
        selectedRows={selectedRows}
        skippedRows={skippedRows}
        diagnostics={props.preview.diagnostics}
        onConfirm={() => {
          setConfirmedRows(selectedRows)
          setConfirmationOpen(false)
        }}
      />
    </div>
  )
}
