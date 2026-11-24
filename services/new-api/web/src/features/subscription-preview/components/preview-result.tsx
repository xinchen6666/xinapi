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

import type { SubscriptionPreview } from '../types'
import { CandidatesTable } from './candidates-table'
import { DiagnosticsTable } from './diagnostics-table'
import { PreviewSummary } from './preview-summary'

type PreviewResultProps = {
  preview: SubscriptionPreview
}

export function PreviewResult(props: PreviewResultProps) {
  const { t } = useTranslation()

  return (
    <div className='flex flex-col gap-4'>
      <PreviewSummary preview={props.preview} />
      <section className='flex flex-col gap-2'>
        <h3 className='text-sm font-medium'>{t('Nodes')}</h3>
        <CandidatesTable candidates={props.preview.candidates} />
      </section>
      <section className='flex flex-col gap-2'>
        <h3 className='text-sm font-medium'>{t('Diagnostics')}</h3>
        <DiagnosticsTable diagnostics={props.preview.diagnostics} />
      </section>
    </div>
  )
}
