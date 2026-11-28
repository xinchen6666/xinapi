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
import { useEffect, useRef, useState } from 'react'
import { useForm } from 'react-hook-form'
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
import { Textarea } from '@/components/ui/textarea'
import { getServerErrorMessage } from '@/lib/server-error-message'

import {
  accountOAuthStartSchema,
  type AccountOAuthStartValues,
} from './lib/oauth-schema'
import { AccountOAuthError, importAccountToken } from './oauth-api'
import { OAuthNodeFields } from './oauth-node-fields'
import type { AccountOAuthReceipt } from './oauth-types'

type Props = {
  userId: number
  sessionId: string
  onClose: () => void
  onImported: (receipt: AccountOAuthReceipt) => void
  onAccessDenied: () => void
}

export function TokenImportDialog(props: Props) {
  const { t } = useTranslation()
  const textarea = useRef<HTMLTextAreaElement>(null)
  const payload = useRef<Parameters<typeof importAccountToken>[0] | null>(null)
  const submitted = useRef(false)
  const [attempted, setAttempted] = useState(false)
  const [errorMessage, setErrorMessage] = useState('')
  const form = useForm<AccountOAuthStartValues>({
    resolver: zodResolver(accountOAuthStartSchema),
    defaultValues: { provider: 'openai', name: '', source_id: '', node_id: '' },
  })
  useEffect(
    () => () => {
      payload.current = null
    },
    []
  )
  const mutation = useMutation({
    mutationFn: async () => {
      const input = payload.current
      payload.current = null
      if (!input) throw new Error('Enter an access token.')
      return importAccountToken(input)
    },
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const submit = form.handleSubmit(async (values) => {
    if (submitted.current) return
    if (!values.source_id || !values.node_id) {
      form.setError(!values.source_id ? 'source_id' : 'node_id', {
        type: 'required',
      })
      return
    }
    const token = textarea.current?.value.trim() ?? ''
    if (!token || token.length > 16384 || /\s/.test(token)) {
      setErrorMessage(t('Enter a single access token, up to 16384 characters.'))
      return
    }
    submitted.current = true
    setAttempted(true)
    setErrorMessage('')
    payload.current = {
      provider: 'openai',
      name: values.name,
      source_id: values.source_id,
      node_id: values.node_id,
      access_token: token,
    }
    if (textarea.current) textarea.current.value = ''
    try {
      props.onImported(await mutation.mutateAsync())
    } catch (error) {
      setErrorMessage(getServerErrorMessage(error))
      if (error instanceof AccountOAuthError && error.accessDenied) {
        props.onAccessDenied()
      }
    } finally {
      payload.current = null
      mutation.reset()
    }
  })
  return (
    <Dialog
      open
      title={t('Import access token')}
      description={t(
        'Connect your OpenAI account through an approved node using an existing access token.'
      )}
      onOpenChange={(open) => {
        if (!open && !mutation.isPending) props.onClose()
      }}
      footer={
        <>
          <Button
            variant='outline'
            disabled={mutation.isPending}
            onClick={props.onClose}
          >
            {t('Close')}
          </Button>
          {!attempted && (
            <Button
              type='submit'
              form='account-token-import'
              disabled={mutation.isPending}
            >
              {t('Import and verify')}
            </Button>
          )}
        </>
      }
    >
      <form id='account-token-import' onSubmit={submit} autoComplete='off'>
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor='token-account-name'>
              {t('Account name')}
            </FieldLabel>
            <Input
              id='token-account-name'
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
          <OAuthNodeFields
            form={form}
            userId={props.userId}
            sessionId={props.sessionId}
            disabled={attempted}
          />
          <Field>
            <FieldLabel htmlFor='account-access-token'>
              {t('Access token')}
            </FieldLabel>
            <Textarea
              ref={textarea}
              id='account-access-token'
              rows={4}
              autoComplete='off'
              spellCheck={false}
              disabled={attempted}
            />
            <FieldDescription>
              {t(
                'Sign in to ChatGPT, open the session page, and copy only the accessToken value. The token is cleared after submission.'
              )}
            </FieldDescription>
            <Button
              type='button'
              variant='outline'
              nativeButton={false}
              render={
                <a
                  href='https://chatgpt.com/api/auth/session'
                  target='_blank'
                  rel='noopener noreferrer'
                  referrerPolicy='no-referrer'
                />
              }
            >
              {t('Open ChatGPT session page')}
            </Button>
          </Field>
          {mutation.isPending && (
            <p role='status' className='text-muted-foreground text-sm'>
              {t('Verifying the token through the selected node…')}
            </p>
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
