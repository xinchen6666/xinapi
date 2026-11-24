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
import { useTranslation } from 'react-i18next'

import { toIntlLocale } from '@/i18n/languages'
import { formatBytes, formatNumber } from '@/lib/format'

import type { SubscriptionPreview } from '../types'

type PreviewSummaryProps = {
  preview: SubscriptionPreview
}

export function PreviewSummary(props: PreviewSummaryProps) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const stats = [
    { label: t('Format'), value: props.preview.format || '—' },
    {
      label: t('Nodes'),
      value: formatNumber(props.preview.candidates.length, locale),
    },
    {
      label: t('Accepted'),
      value: formatNumber(props.preview.accepted, locale),
    },
    {
      label: t('Rejected'),
      value: formatNumber(props.preview.rejected, locale),
    },
    {
      label: t('Downloaded'),
      value: formatBytes(props.preview.fetched_bytes, locale),
    },
  ]

  return (
    <dl className='grid grid-cols-2 gap-2 sm:grid-cols-5'>
      {stats.map((stat) => (
        <div key={stat.label} className='rounded-md border px-3 py-2'>
          <dt className='text-muted-foreground text-xs'>{stat.label}</dt>
          <dd className='truncate text-sm font-medium'>{stat.value}</dd>
        </div>
      ))}
    </dl>
  )
}
