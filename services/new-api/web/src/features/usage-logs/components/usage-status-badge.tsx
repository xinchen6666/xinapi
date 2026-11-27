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

import { StatusBadge, type StatusVariant } from '@/components/status-badge'

import type { LogOtherData } from '../types'

export function UsageStatusBadge(props: { other: LogOtherData | null }) {
  const { t } = useTranslation()
  let label: string
  let variant: StatusVariant
  switch (props.other?.usage_status) {
    case 'confirmed':
      label = t('Confirmed usage')
      variant = 'success'
      break
    case 'estimated':
      label = t('Estimated usage')
      variant = 'warning'
      break
    case 'unknown':
      label = t('Usage unknown')
      variant = 'neutral'
      break
    default:
      if (props.other?.request_path !== '/v1/responses') return null
      label = t('Usage unknown')
      variant = 'neutral'
  }
  return (
    <StatusBadge
      label={label}
      variant={variant}
      type='badge'
      size='sm'
      copyable={false}
      className='h-5 text-[11px]'
    />
  )
}
