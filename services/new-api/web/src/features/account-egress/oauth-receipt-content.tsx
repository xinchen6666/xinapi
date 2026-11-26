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
import { ErrorState } from '@/components/error-state'
import { LoadingState } from '@/components/loading-state'
import { Alert, AlertDescription } from '@/components/ui/alert'
import { Button } from '@/components/ui/button'
import {
  Field,
  FieldDescription,
  FieldError,
  FieldGroup,
  FieldLabel,
} from '@/components/ui/field'
import { Textarea } from '@/components/ui/textarea'

import type { useOAuthReceipt } from './hooks/use-oauth-receipt'
import { OAuthReceiptFacts } from './oauth-receipt-facts'

export function OAuthReceiptContent(props: {
  state: ReturnType<typeof useOAuthReceipt>
}) {
  const { t } = useTranslation()
  const {
    receipt,
    attempted,
    link,
    expired,
    canComplete,
    pending,
    form,
    prepareCompletion,
    errorMessage,
  } = props.state
  return (
    <>
      {receipt.isPending && <LoadingState />}
      {receipt.isError && (
        <ErrorState
          className='min-h-32'
          title={t('Account authorization operation is unavailable')}
          onRetry={() => void receipt.refetch()}
        />
      )}
      {receipt.data && !receipt.isError && (
        <FieldGroup>
          <OAuthReceiptFacts receipt={receipt.data} />
          {receipt.data.status === 'pending' &&
            link &&
            !expired &&
            !attempted && (
              <div className='flex flex-wrap gap-2'>
                <Button
                  nativeButton={false}
                  render={
                    <a
                      href={link}
                      target='_blank'
                      rel='noopener noreferrer'
                      referrerPolicy='no-referrer'
                    />
                  }
                >
                  {t('Open provider authorization')}
                </Button>
                <CopyButton
                  value={link}
                  size='sm'
                  variant='outline'
                  tooltip={t('Copy authorization link')}
                >
                  {t('Copy authorization link')}
                </CopyButton>
              </div>
            )}
          {receipt.data.status === 'pending' && !link && !attempted && (
            <Alert>
              <AlertDescription>
                {t(
                  'Continue in the provider tab already opened for this operation, then paste its callback URL here.'
                )}
              </AlertDescription>
            </Alert>
          )}
          {receipt.data.status === 'completing' && (
            <Alert>
              <AlertDescription>
                {t(
                  'The authorization is processing. This page only refreshes the receipt.'
                )}
              </AlertDescription>
            </Alert>
          )}
          {(attempted || receipt.data.status === 'unknown') &&
            receipt.data.status !== 'completed' && (
              <Alert>
                <AlertDescription>
                  {t(
                    'The callback was cleared. Refresh the receipt to confirm the outcome, or start a new authorization.'
                  )}
                </AlertDescription>
              </Alert>
            )}
          {expired && receipt.data.status === 'pending' && (
            <Alert>
              <AlertDescription>
                {t(
                  'This authorization has expired. Start a new authorization.'
                )}
              </AlertDescription>
            </Alert>
          )}
          {canComplete && (
            <form
              id='account-oauth-callback'
              onSubmit={prepareCompletion}
              autoComplete='off'
            >
              <FieldGroup>
                <Field data-invalid={!!form.formState.errors.callback_url}>
                  <FieldLabel htmlFor='oauth-callback-url'>
                    {t('Authorization callback URL')}
                  </FieldLabel>
                  <Textarea
                    id='oauth-callback-url'
                    rows={3}
                    autoComplete='off'
                    spellCheck={false}
                    disabled={pending}
                    {...form.register('callback_url')}
                    aria-invalid={!!form.formState.errors.callback_url}
                  />
                  <FieldDescription>
                    {t(
                      'Paste the full callback URL. It is cleared immediately after this single submission.'
                    )}
                  </FieldDescription>
                  {form.formState.errors.callback_url && (
                    <FieldError>
                      {t('Enter a complete HTTP or HTTPS callback URL.')}
                    </FieldError>
                  )}
                </Field>
                <div className='flex justify-end'>
                  <Button type='submit' disabled={pending}>
                    {t('Submit authorization callback')}
                  </Button>
                </div>
              </FieldGroup>
            </form>
          )}
          {errorMessage && (
            <Alert variant='destructive'>
              <AlertDescription>{errorMessage}</AlertDescription>
            </Alert>
          )}
        </FieldGroup>
      )}
    </>
  )
}
