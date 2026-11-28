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

import { CopyButton } from '@/components/copy-button'
import { Badge } from '@/components/ui/badge'
import { toIntlLocale } from '@/i18n/languages'
import { formatNumber, formatTimestampToDate } from '@/lib/format'

import type { AccountOAuthReceipt, AccountOAuthStatus } from './oauth-types'

export function OAuthStatus(props: { status: AccountOAuthStatus }) {
  const { t } = useTranslation()
  const labels = {
    pending: t('Authorization pending'),
    completing: t('Authorization completing'),
    completed: t('Authorization completed'),
    failed: t('Authorization failed'),
    unknown: t('Authorization result unknown'),
    cancelled: t('Authorization cancelled'),
    expired: t('Authorization expired'),
  }
  return (
    <Badge variant={props.status === 'failed' ? 'destructive' : 'secondary'}>
      {labels[props.status] ?? t('Authorization result unknown')}
    </Badge>
  )
}

export function OAuthReceiptFacts(props: { receipt: AccountOAuthReceipt }) {
  const { t, i18n } = useTranslation()
  const locale = toIntlLocale(i18n.resolvedLanguage || i18n.language)
  const receipt = props.receipt
  const fields = [
    [t('Provider'), receipt.provider === 'openai' ? 'OpenAI' : 'xAI'],
    [
      t('Account ID'),
      receipt.status === 'completed' ? receipt.account_id || '—' : '—',
    ],
    [t('Approved source'), receipt.source_id || '—'],
    [t('Approved node'), receipt.node_id || '—'],
    [t('Connection version'), receipt.version_id || '—'],
    [t('Egress ID'), receipt.proxy_id || '—'],
    [t('Route generation'), formatNumber(receipt.route_generation, locale)],
    [
      t('Expires at'),
      formatTimestampToDate(Date.parse(receipt.expires_at), 'milliseconds'),
    ],
  ]
  return (
    <div className='flex flex-col gap-4'>
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <OAuthStatus status={receipt.status} />
        <CopyButton
          value={receipt.operation_id}
          size='sm'
          variant='outline'
          tooltip={t('Copy operation ID')}
        >
          {t('Copy operation ID')}
        </CopyButton>
      </div>
      <dl className='grid grid-cols-1 gap-3 text-sm sm:grid-cols-2'>
        <div className='sm:col-span-2'>
          <dt className='text-muted-foreground'>
            {t('Authorization operation ID')}
          </dt>
          <dd className='font-mono text-xs break-all'>
            {receipt.operation_id}
          </dd>
        </div>
        {fields.map(([label, value]) => (
          <div key={label}>
            <dt className='text-muted-foreground'>{label}</dt>
            <dd className='break-all'>{value}</dd>
          </div>
        ))}
      </dl>
    </div>
  )
}
