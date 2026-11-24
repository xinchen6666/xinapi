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
import { Eye } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
} from '@/components/ui/field'
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
  InputGroupInput,
} from '@/components/ui/input-group'
import { Spinner } from '@/components/ui/spinner'

type SourceFormProps = {
  value: string
  /** Explained in the field, next to the link the administrator pasted. */
  error: string
  pending: boolean
  onChange: (value: string) => void
  onError: (message: string) => void
  onSubmit: () => void
}

export function SourceForm(props: SourceFormProps) {
  const { t } = useTranslation()

  return (
    <form
      onSubmit={(event) => {
        event.preventDefault()
        props.onSubmit()
      }}
    >
      <Field data-invalid={props.error ? true : undefined}>
        <FieldLabel htmlFor='subscription-source'>
          {t('Subscription link')}
        </FieldLabel>
        <InputGroup>
          <InputGroupInput
            id='subscription-source'
            type='url'
            inputMode='url'
            autoComplete='off'
            spellCheck={false}
            value={props.value}
            placeholder='https://example.com/subscription.yaml'
            aria-describedby='subscription-source-hint'
            aria-invalid={props.error ? true : undefined}
            onChange={(event) => {
              props.onChange(event.target.value)
              props.onError('')
            }}
          />
          <InputGroupAddon align='inline-end'>
            <InputGroupButton
              type='submit'
              variant='outline'
              disabled={props.pending || props.value.trim() === ''}
            >
              {props.pending ? (
                <Spinner aria-hidden='true' className='size-3.5' />
              ) : (
                <Eye aria-hidden='true' />
              )}
              {props.pending ? t('Previewing...') : t('Preview')}
            </InputGroupButton>
          </InputGroupAddon>
        </InputGroup>
        <FieldDescription id='subscription-source-hint'>
          {t(
            'The link is downloaded and parsed for review. Saving starts in the next step.'
          )}
        </FieldDescription>
        {props.error ? <FieldError>{props.error}</FieldError> : null}
      </Field>
    </form>
  )
}
