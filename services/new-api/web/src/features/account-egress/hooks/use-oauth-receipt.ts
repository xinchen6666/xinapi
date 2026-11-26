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
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'
import { useForm } from 'react-hook-form'

import {
  getServerErrorMessage,
  getServerErrorStatus,
} from '@/lib/server-error-message'

import {
  accountAuthorizationLink,
  accountOAuthCallbackSchema,
  type AccountOAuthCallbackValues,
} from '../lib/oauth-schema'
import {
  AccountOAuthError,
  cancelAccountOAuth,
  completeAccountOAuth,
  getAccountOAuthReceipt,
} from '../oauth-api'
import type { AccountOAuthReceipt } from '../oauth-types'

type Props = {
  operationId: string
  authorizationUrl?: string
  userId: number
  sessionId: string
  onAccessDenied: () => void
}

export function useOAuthReceipt(props: Props) {
  const queryClient = useQueryClient()
  const callback = useRef('')
  const submitted = useRef(false)
  const [attempted, setAttempted] = useState(false)
  const [confirmation, setConfirmation] = useState<
    'complete' | 'cancel' | null
  >(null)
  const [errorMessage, setErrorMessage] = useState('')
  const form = useForm<AccountOAuthCallbackValues>({
    resolver: zodResolver(accountOAuthCallbackSchema),
    defaultValues: { callback_url: '' },
  })
  const queryKey = [
    'account-egress',
    props.userId,
    'oauth',
    props.sessionId,
    'receipt',
    props.operationId,
  ] as const
  const receipt = useQuery({
    queryKey,
    queryFn: ({ signal }) => getAccountOAuthReceipt(props.operationId, signal),
    retry: false,
    gcTime: 0,
    refetchInterval: (query) =>
      query.state.data &&
      ['pending', 'completing'].includes(query.state.data.status) &&
      Date.parse(query.state.data.expires_at) > Date.now()
        ? 3000
        : false,
    meta: { errorToast: false },
  })
  const onAccessDenied = props.onAccessDenied
  useEffect(() => {
    if (getServerErrorStatus(receipt.error) === 403) onAccessDenied()
  }, [receipt.error, onAccessDenied])
  useEffect(
    () => () => {
      callback.current = ''
    },
    []
  )

  const updateReceipt = (next: AccountOAuthReceipt) => {
    queryClient.setQueryData(queryKey, next)
    void queryClient.invalidateQueries({
      queryKey: ['account-egress', props.userId],
    })
  }
  const complete = useMutation({
    // 回调只在短期ref中持有，mutation变量和查询缓存仅包含回执标识
    mutationFn: async () => {
      const value = callback.current
      callback.current = ''
      return completeAccountOAuth(props.operationId, value)
    },
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const cancelSubmitted = useRef(false)
  const cancel = useMutation({
    mutationFn: () => cancelAccountOAuth(props.operationId),
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const pending = complete.isPending || cancel.isPending
  const expiresAt = Date.parse(receipt.data?.expires_at ?? '')
  const expired = Number.isFinite(expiresAt) && expiresAt <= Date.now()
  const canComplete =
    receipt.data?.status === 'pending' && !expired && !attempted
  const link = receipt.data
    ? accountAuthorizationLink(props.authorizationUrl, receipt.data.provider)
    : null

  const confirmCompletion = async () => {
    if (submitted.current || !canComplete) return
    submitted.current = true
    setAttempted(true)
    setConfirmation(null)
    setErrorMessage('')
    form.reset({ callback_url: '' })
    try {
      updateReceipt(await complete.mutateAsync())
    } catch (error) {
      setErrorMessage(getServerErrorMessage(error))
      if (error instanceof AccountOAuthError && error.accessDenied) {
        props.onAccessDenied()
      } else {
        void receipt.refetch()
      }
    } finally {
      callback.current = ''
      form.reset({ callback_url: '' })
      complete.reset()
    }
  }
  const confirmCancel = async () => {
    if (cancelSubmitted.current || pending) return
    cancelSubmitted.current = true
    setConfirmation(null)
    setErrorMessage('')
    callback.current = ''
    form.reset({ callback_url: '' })
    try {
      updateReceipt(await cancel.mutateAsync())
    } catch (error) {
      setErrorMessage(getServerErrorMessage(error))
      if (error instanceof AccountOAuthError && error.accessDenied) {
        props.onAccessDenied()
      } else {
        void receipt.refetch()
      }
    } finally {
      cancelSubmitted.current = false
      cancel.reset()
    }
  }
  const prepareCompletion = form.handleSubmit((values) => {
    if (!canComplete || pending) return
    callback.current = values.callback_url
    setConfirmation('complete')
  })
  const restart =
    receipt.data &&
    ((expired && receipt.data.status === 'pending') ||
      ['unknown', 'failed', 'cancelled'].includes(receipt.data.status))

  return {
    receipt,
    form,
    attempted,
    confirmation,
    setConfirmation,
    errorMessage,
    pending,
    expired,
    canComplete,
    link,
    confirmCompletion,
    confirmCancel,
    prepareCompletion,
    restart,
    closeConfirmation: () => {
      setConfirmation(null)
      callback.current = ''
    },
  }
}
