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

import { ConfirmDialog } from '@/components/confirm-dialog'
import { Dialog } from '@/components/dialog'
import { Button } from '@/components/ui/button'

import { useOAuthReceipt } from './hooks/use-oauth-receipt'
import { OAuthReceiptContent } from './oauth-receipt-content'

type Props = {
  operationId: string
  authorizationUrl?: string
  userId: number
  sessionId: string
  onClose: () => void
  onAccessDenied: () => void
  onNewAuthorization: () => void
}

export function OAuthReceiptDialog(props: Props) {
  const { t } = useTranslation()
  const state = useOAuthReceipt(props)
  const {
    receipt,
    pending,
    restart,
    confirmation,
    setConfirmation,
    canComplete,
    confirmCompletion,
    confirmCancel,
  } = state
  return (
    <>
      <Dialog
        open
        title={t('Authorization receipt')}
        description={t(
          'Receipts belong to the dashboard login session that started the authorization.'
        )}
        onOpenChange={(open) => {
          if (!open && !pending) props.onClose()
        }}
        footer={
          <>
            <Button
              type='button'
              variant='outline'
              disabled={pending}
              onClick={props.onClose}
            >
              {t('Close')}
            </Button>
            <Button
              type='button'
              variant='outline'
              disabled={pending || receipt.isFetching}
              onClick={() => void receipt.refetch()}
            >
              {t('Refresh receipt')}
            </Button>
            {receipt.data &&
              ['pending', 'unknown', 'expired'].includes(
                receipt.data.status
              ) && (
                <Button
                  type='button'
                  variant='outline'
                  disabled={pending}
                  onClick={() => setConfirmation('cancel')}
                >
                  {t('Cancel authorization')}
                </Button>
              )}
            {restart && (
              <Button
                type='button'
                disabled={pending}
                onClick={props.onNewAuthorization}
              >
                {t('Start new authorization')}
              </Button>
            )}
          </>
        }
      >
        <OAuthReceiptContent state={state} />
      </Dialog>
      <ConfirmDialog
        open={confirmation === 'complete'}
        onOpenChange={(open) => {
          if (!open) {
            state.closeConfirmation()
          }
        }}
        title={t('Submit this callback once?')}
        desc={t(
          'The server exchanges this callback once. If the connection is interrupted, use the receipt to check the outcome.'
        )}
        confirmText={t('Submit once')}
        disabled={!canComplete}
        isLoading={pending}
        handleConfirm={() => void confirmCompletion()}
      />
      <ConfirmDialog
        open={confirmation === 'cancel'}
        onOpenChange={(open) => {
          if (!open) setConfirmation(null)
        }}
        title={t('Cancel authorization?')}
        desc={t(
          'Cancel the pending authorization and release its reserved egress.'
        )}
        confirmText={t('Cancel authorization')}
        destructive
        isLoading={pending}
        handleConfirm={() => void confirmCancel()}
      />
    </>
  )
}
