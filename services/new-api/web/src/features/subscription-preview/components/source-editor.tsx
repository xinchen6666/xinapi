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
import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation } from '@tanstack/react-query'
import { useRef } from 'react'
import { useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'
import { z } from 'zod'

import { Dialog } from '@/components/dialog'
import { Button } from '@/components/ui/button'
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
} from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { getServerErrorMessage } from '@/lib/server-error-message'

import { normalizeSubscriptionSource } from '../lib/subscription-preview'
import { saveSource } from '../source-api'
import type { SourceDetail } from '../source-types'

const schema = z.object({
  name: z
    .string()
    .trim()
    .min(1)
    .refine((value) => [...value].length <= 100),
  source_url: z
    .string()
    .trim()
    .refine(
      (value) =>
        value === '' ||
        (new TextEncoder().encode(value).length <= 2048 &&
          normalizeSubscriptionSource(value) !== null)
    ),
})
type Values = z.infer<typeof schema>
type Props = {
  detail?: SourceDetail
  onClose: () => void
  onSaved: (detail: SourceDetail) => void
  onConflict: () => void
}

export function SourceEditor(props: Props) {
  const { t } = useTranslation()
  // 草稿绑定打开时的版本；后台刷新推进版本后，提交仍按原版本校验并得到冲突
  const openingRevision = useRef(props.detail?.source.revision)
  const form = useForm<Values>({
    resolver: zodResolver(schema),
    defaultValues: { name: props.detail?.source.name ?? '', source_url: '' },
  })
  const save = useMutation({
    mutationFn: saveSource,
    meta: { errorToast: false },
    onSuccess: (detail) => {
      form.reset({ name: detail.source.name, source_url: '' })
      props.onSaved(detail)
    },
    onError: props.onConflict,
  })
  const submit = form.handleSubmit((values) => {
    if (!props.detail && !values.source_url) {
      form.setError('source_url', { message: t('Enter a subscription link.') })
      return
    }
    const input: Parameters<typeof saveSource>[0] = { name: values.name }
    if (values.source_url) input.source_url = values.source_url
    if (props.detail) {
      input.source_id = props.detail.source.id
      input.expected_revision = openingRevision.current
    }
    save.mutate(input)
    form.setValue('source_url', '')
  })
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !save.isPending) props.onClose()
      }}
      title={
        props.detail
          ? t('Edit subscription source')
          : t('Create subscription source')
      }
      description={t(
        'The subscription link is submitted privately. A blank replacement keeps the saved link.'
      )}
    >
      <form onSubmit={submit} className='space-y-4'>
        <Field>
          <FieldLabel htmlFor='saved-source-name'>
            {t('Source name')}
          </FieldLabel>
          <Input
            id='saved-source-name'
            autoComplete='off'
            {...form.register('name')}
            aria-invalid={!!form.formState.errors.name}
          />
          {form.formState.errors.name ? (
            <FieldError>
              {t('Enter a source name with 1 to 100 characters.')}
            </FieldError>
          ) : null}
        </Field>
        <Field>
          <FieldLabel htmlFor='saved-source-url'>
            {props.detail
              ? t('Replacement subscription link')
              : t('Subscription link')}
          </FieldLabel>
          <Input
            id='saved-source-url'
            type='password'
            autoComplete='new-password'
            spellCheck={false}
            {...form.register('source_url')}
            aria-invalid={!!form.formState.errors.source_url}
          />
          <FieldDescription>
            {t(
              'Only the submitted link is used. Source lists contain display metadata.'
            )}
          </FieldDescription>
          {form.formState.errors.source_url ? (
            <FieldError>
              {form.formState.errors.source_url.type === 'custom'
                ? t('Enter an absolute http(s) URL.')
                : form.formState.errors.source_url.message}
            </FieldError>
          ) : null}
        </Field>
        {save.isError ? (
          <FieldError>{getServerErrorMessage(save.error)}</FieldError>
        ) : null}
        <div className='flex justify-end gap-2'>
          <Button
            type='button'
            variant='outline'
            disabled={save.isPending}
            onClick={props.onClose}
          >
            {t('Cancel')}
          </Button>
          <Button type='submit' disabled={save.isPending}>
            {t('Save source')}
          </Button>
        </div>
      </form>
    </Dialog>
  )
}
