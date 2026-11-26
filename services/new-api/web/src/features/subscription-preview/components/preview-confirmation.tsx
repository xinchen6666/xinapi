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
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { ConfirmDialog } from '@/components/confirm-dialog'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber } from '@/lib/format'

import {
  buildDiagnosticRows,
  diagnosticMessage,
} from '../lib/subscription-preview'
import type { CandidateRow, SubscriptionPreviewDiagnostic } from '../types'

type PreviewConfirmationProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  selectedRows: CandidateRow[]
  skippedRows: CandidateRow[]
  diagnostics: SubscriptionPreviewDiagnostic[]
  onConfirm: () => void
}

export function PreviewConfirmation(props: PreviewConfirmationProps) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const diagnosticRows = useMemo(
    () => buildDiagnosticRows(props.diagnostics),
    [props.diagnostics]
  )

  return (
    <ConfirmDialog
      open={props.open}
      onOpenChange={props.onOpenChange}
      title={t('Confirm preview scope')}
      desc={t(
        'Confirmation stays in this preview. Accounts and runtime configuration stay unchanged.'
      )}
      confirmText={t('Confirm selected metadata')}
      disabled={props.selectedRows.length === 0}
      handleConfirm={props.onConfirm}
      className='sm:max-w-xl'
    >
      <div className='max-h-[55dvh] space-y-4 overflow-y-auto text-sm'>
        <section className='space-y-2'>
          <h3 className='font-medium'>
            {t('Selected nodes: {{count}}', {
              count: formatNumber(props.selectedRows.length, locale),
            })}
          </h3>
          <ul className='list-disc space-y-1 pl-5'>
            {props.selectedRows.map((row) => (
              <li key={row.id} className='break-all'>
                {row.name || '—'} · {row.protocol} ·{' '}
                {row.supported ? t('Native HTTP/SOCKS') : t('Mihomo pending')}
              </li>
            ))}
          </ul>
        </section>
        <section className='space-y-2'>
          <h3 className='font-medium'>
            {t('Skipped nodes: {{count}}', {
              count: formatNumber(props.skippedRows.length, locale),
            })}
          </h3>
          <ul className='list-disc space-y-1 pl-5'>
            {props.skippedRows.map((row) => (
              <li key={row.id} className='break-all'>
                {row.name || '—'} · {row.protocol} ·{' '}
                {row.parse_valid
                  ? t('Outside selected scope')
                  : t('Needs correction')}
              </li>
            ))}
          </ul>
        </section>
        <section className='space-y-2'>
          <h3 className='font-medium'>
            {t('Diagnostics to acknowledge: {{count}}', {
              count: formatNumber(props.diagnostics.length, locale),
            })}
          </h3>
          <ul className='space-y-2'>
            {diagnosticRows.map((diagnostic) => (
              <li key={diagnostic.id} className='break-words'>
                <span className='font-medium'>{diagnostic.subject || '—'}</span>
                {' · '}
                {diagnostic.code}
                <p className='text-muted-foreground'>
                  {diagnosticMessage(diagnostic, t)}
                </p>
              </li>
            ))}
          </ul>
        </section>
      </div>
    </ConfirmDialog>
  )
}
