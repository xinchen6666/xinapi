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
import { useRef, useState } from 'react'
import { Controller, useForm } from 'react-hook-form'
import { useTranslation } from 'react-i18next'

import { Dialog } from '@/components/dialog'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { getServerErrorMessage } from '@/lib/server-error-message'

import {
  accountOAuthStartSchema,
  type AccountOAuthStartValues,
} from './lib/oauth-schema'
import { AccountOAuthError, startAccountOAuth } from './oauth-api'
import { OAuthNodeFields } from './oauth-node-fields'
import type { AccountOAuthStarted, AccountOAuthTarget } from './oauth-types'

type Props = {
  target?: AccountOAuthTarget
  userId: number
  sessionId: string
  onClose: () => void
  onStarted: (receipt: AccountOAuthStarted) => void
  onReadReceipts: () => void
  onAccessDenied: () => void
}

export function OAuthStartDialog(props: Props) {
  const { t } = useTranslation()
  const submitted = useRef(false)
  const [attempted, setAttempted] = useState(false)
  const [errorMessage, setErrorMessage] = useState('')
  const form = useForm<AccountOAuthStartValues>({
    resolver: zodResolver(accountOAuthStartSchema),
    defaultValues: {
      provider: props.target?.provider ?? 'openai',
      name: props.target?.name ?? '',
      source_id: '',
      node_id: '',
    },
  })
  const start = useMutation({
    mutationFn: startAccountOAuth,
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const submit = form.handleSubmit(async (values) => {
    if (submitted.current) return
    if (!props.target && (!values.source_id || !values.node_id)) {
      form.setError(!values.source_id ? 'source_id' : 'node_id', {
        type: 'required',
      })
      return
    }
    submitted.current = true
    setAttempted(true)
    try {
      const receipt = await start.mutateAsync({
        provider: props.target?.provider ?? values.provider,
        name: values.name,
        ...(props.target
          ? { account_id: props.target.account_id }
          : { source_id: values.source_id, node_id: values.node_id }),
      })
      props.onStarted(receipt)
    } catch (error) {
      setErrorMessage(getServerErrorMessage(error))
      if (error instanceof AccountOAuthError && error.accessDenied) {
        props.onAccessDenied()
      }
    } finally {
      start.reset()
    }
  })
  const providers = [
    { value: 'openai', label: 'OpenAI' },
    { value: 'xai', label: 'xAI' },
  ]
  return (
    <Dialog
      open
      title={
        props.target
          ? t('Reauthorize upstream account')
          : t('Start account authorization')
      }
      description={t(
        'Your browser opens the provider page using its own network. Server-side exchange uses the account egress.'
      )}
      onOpenChange={(open) => {
        if (!open && !start.isPending) props.onClose()
      }}
      footer={
        <>
          <Button
            type='button'
            variant='outline'
            disabled={start.isPending}
            onClick={props.onClose}
          >
            {t('Close')}
          </Button>
          {attempted ? (
            <Button
              type='button'
              disabled={start.isPending}
              onClick={props.onReadReceipts}
            >
              {t('View authorization receipts')}
            </Button>
          ) : (
            <Button
              type='submit'
              form='account-oauth-start'
              disabled={start.isPending}
            >
              {t('Start authorization')}
            </Button>
          )}
        </>
      }
    >
      <form id='account-oauth-start' onSubmit={submit}>
        <FieldGroup>
          <Controller
            control={form.control}
            name='provider'
            render={({ field }) => (
              <Field data-disabled={!!props.target || attempted}>
                <FieldLabel htmlFor='oauth-provider'>
                  {t('Provider')}
                </FieldLabel>
                <Select
                  items={providers}
                  value={field.value}
                  onValueChange={(value) => field.onChange(value)}
                  disabled={!!props.target || attempted}
                >
                  <SelectTrigger id='oauth-provider' className='w-full'>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectGroup>
                      {providers.map((item) => (
                        <SelectItem key={item.value} value={item.value}>
                          {item.label}
                        </SelectItem>
                      ))}
                    </SelectGroup>
                  </SelectContent>
                </Select>
              </Field>
            )}
          />
          <Field
            data-invalid={!!form.formState.errors.name}
            data-disabled={attempted}
          >
            <FieldLabel htmlFor='oauth-name'>{t('Account name')}</FieldLabel>
            <Input
              id='oauth-name'
              autoComplete='off'
              disabled={attempted}
              {...form.register('name')}
              aria-invalid={!!form.formState.errors.name}
            />
            {form.formState.errors.name && (
              <FieldError>
                {t('Enter an account name with 1 to 100 characters.')}
              </FieldError>
            )}
          </Field>
          {props.target ? (
            <Field>
              <FieldLabel htmlFor='oauth-account-id'>
                {t('Account ID')}
              </FieldLabel>
              <Input
                id='oauth-account-id'
                value={props.target.account_id}
                readOnly
              />
              <FieldDescription>
                {t(
                  'Reauthorization uses the existing account and its current managed egress.'
                )}
              </FieldDescription>
            </Field>
          ) : (
            <OAuthNodeFields
              form={form}
              userId={props.userId}
              sessionId={props.sessionId}
              disabled={attempted}
            />
          )}
          {errorMessage && (
            <Alert variant='destructive'>
              <AlertDescription>{errorMessage}</AlertDescription>
            </Alert>
          )}
          {attempted && errorMessage && (
            <Alert>
              <AlertDescription>
                {t(
                  'Read the session receipt to confirm the result before starting another authorization.'
                )}
              </AlertDescription>
            </Alert>
          )}
        </FieldGroup>
      </form>
    </Dialog>
  )
}
