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
import { useQuery } from '@tanstack/react-query'
import { Controller, useWatch, type UseFormReturn } from 'react-hook-form'
import { useTranslation } from 'react-i18next'

import { ErrorState } from '@/components/error-state'
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
} from '@/components/ui/field'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  listSources,
  sourceDetail,
} from '@/features/subscription-preview/source-api'

import type { AccountOAuthStartValues } from './lib/oauth-schema'

type Props = {
  form: UseFormReturn<AccountOAuthStartValues>
  userId: number
  sessionId: string
  disabled: boolean
}

export function OAuthNodeFields(props: Props) {
  const { t } = useTranslation()
  const sourceId = useWatch({ control: props.form.control, name: 'source_id' })
  const sources = useQuery({
    queryKey: [
      'account-egress',
      props.userId,
      'oauth',
      props.sessionId,
      'sources',
    ],
    queryFn: ({ signal }) => listSources(signal),
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const detail = useQuery({
    queryKey: [
      'account-egress',
      props.userId,
      'oauth',
      props.sessionId,
      'source',
      sourceId,
    ],
    queryFn: ({ signal }) => sourceDetail(sourceId, signal),
    enabled: !!sourceId,
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const sourceItems = [
    { value: '', label: t('Select an approved source') },
    ...(sources.data ?? [])
      .filter((item) => item.approved_version > 0 && item.node_count > 0)
      .map((item) => ({ value: item.id, label: item.name })),
  ]
  const nodeItems = [
    { value: '', label: t('Select an approved node') },
    ...(detail.data?.nodes ?? [])
      .filter((node) => node.available && !!node.version_id)
      .map((node) => ({ value: node.id, label: node.name })),
  ]
  if (sources.isError || detail.isError) {
    return (
      <ErrorState
        className='min-h-32'
        title={t('Approved nodes are unavailable')}
        onRetry={() => {
          void sources.refetch()
          if (sourceId) void detail.refetch()
        }}
      />
    )
  }
  return (
    <>
      <Controller
        control={props.form.control}
        name='source_id'
        render={({ field, fieldState }) => (
          <Field
            data-invalid={!!fieldState.error}
            data-disabled={props.disabled || sources.isPending}
          >
            <FieldLabel htmlFor='oauth-source'>
              {t('Approved source')}
            </FieldLabel>
            <Select
              items={sourceItems}
              value={field.value}
              onValueChange={(value) => {
                field.onChange(value ?? '')
                props.form.setValue('node_id', '')
              }}
              disabled={props.disabled || sources.isPending}
            >
              <SelectTrigger
                id='oauth-source'
                className='w-full'
                aria-invalid={!!fieldState.error}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {sourceItems.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {item.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            {fieldState.error && (
              <FieldError>
                {t('Select an approved source and an available node.')}
              </FieldError>
            )}
          </Field>
        )}
      />
      <Controller
        control={props.form.control}
        name='node_id'
        render={({ field, fieldState }) => (
          <Field
            data-invalid={!!fieldState.error}
            data-disabled={props.disabled || !sourceId || detail.isPending}
          >
            <FieldLabel htmlFor='oauth-node'>{t('Approved node')}</FieldLabel>
            <Select
              items={nodeItems}
              value={field.value}
              onValueChange={(value) => field.onChange(value ?? '')}
              disabled={props.disabled || !sourceId || detail.isPending}
            >
              <SelectTrigger
                id='oauth-node'
                className='w-full'
                aria-invalid={!!fieldState.error}
              >
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  {nodeItems.map((item) => (
                    <SelectItem key={item.value} value={item.value}>
                      {item.label}
                    </SelectItem>
                  ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <FieldDescription>
              {t(
                'The selected approved node is reserved for this authorization.'
              )}
            </FieldDescription>
            {fieldState.error && (
              <FieldError>
                {t('Select an approved source and an available node.')}
              </FieldError>
            )}
            {sourceId && detail.isSuccess && nodeItems.length === 1 && (
              <FieldDescription>
                {t('Approve an available node in Subscription Preview first.')}
              </FieldDescription>
            )}
          </Field>
        )}
      />
    </>
  )
}
