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
import { useMutation } from '@tanstack/react-query'
import { Rss } from 'lucide-react'
import { useCallback, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { EmptyState } from '@/components/empty-state'
import { ErrorState } from '@/components/error-state'
import { SectionPageLayout } from '@/components/layout'
import { LoadingState } from '@/components/loading-state'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import {
  ADMIN_PERMISSION_RESOURCES,
  ADMIN_PERMISSION_ACTIONS,
  hasPermission,
} from '@/lib/admin-permissions'
import { ROLE } from '@/lib/roles'
import { getServerErrorMessage } from '@/lib/server-error-message'
import { useAuthStore } from '@/stores/auth-store'

import { previewSubscription } from './api'
import { PreviewResult } from './components/preview-result'
import { SourceForm } from './components/source-form'
import { SourceManagement } from './components/source-management'
import {
  SUBSCRIPTION_SOURCE_MAX_LENGTH,
  normalizeSubscriptionSource,
  previewErrorCode,
} from './lib/subscription-preview'

export function SubscriptionPreview() {
  const userId = useAuthStore((state) => state.auth.user?.id)
  return <SubscriptionPreviewSession key={userId ?? 'anonymous'} />
}

function SubscriptionPreviewSession() {
  const { t } = useTranslation()
  const user = useAuthStore((state) => state.auth.user)
  const [source, setSource] = useState('')
  const [sourceError, setSourceError] = useState('')
  const [accessRevoked, setAccessRevoked] = useState(false)
  const userId = user?.id
  const canRead =
    !!user &&
    user.role >= ROLE.ADMIN &&
    hasPermission(
      user,
      ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
      ADMIN_PERMISSION_ACTIONS.READ
    )

  const canManageSources = hasPermission(
    user,
    ADMIN_PERMISSION_RESOURCES.ACCOUNT_EGRESS,
    ADMIN_PERMISSION_ACTIONS.MANAGE_SOURCES
  )

  const handleReadDenied = useCallback(() => {
    setAccessRevoked(true)
    const current = useAuthStore.getState().auth.user
    if (current && current.id === userId) {
      useAuthStore.getState().auth.setUser({
        ...current,
        permissions: {
          ...current.permissions,
          admin_permissions: {
            ...current.permissions?.admin_permissions,
            account_egress: {
              ...current.permissions?.admin_permissions?.account_egress,
              read: false,
            },
          },
        },
      })
    }
  }, [userId])

  const previewMutation = useMutation({
    meta: { errorToast: false },
    mutationFn: previewSubscription,
  })

  const submit = () => {
    if (!canManageSources) return
    previewMutation.reset()
    const trimmed = source.trim()
    if (trimmed === '') {
      setSourceError(t('Enter a subscription link.'))
      return
    }
    if (trimmed.length > SUBSCRIPTION_SOURCE_MAX_LENGTH) {
      setSourceError(
        t('The subscription link must be at most {{max}} characters', {
          max: SUBSCRIPTION_SOURCE_MAX_LENGTH,
        })
      )
      return
    }
    const normalized = normalizeSubscriptionSource(trimmed)
    if (!normalized) {
      setSourceError(t('Enter an absolute http(s) URL.'))
      return
    }
    setSourceError('')
    previewMutation.mutate(normalized)
  }

  return (
    <SectionPageLayout stackActionsOnMobile>
      <SectionPageLayout.Title>
        {t('Subscription Preview')}
      </SectionPageLayout.Title>
      <SectionPageLayout.Content>
        {canRead && !accessRevoked ? (
          <Tabs
            defaultValue={canManageSources ? 'preview' : 'sources'}
            className='min-h-0'
          >
            <TabsList aria-label={t('Subscription workspace')}>
              <TabsTrigger value='preview'>
                {t('Subscription Preview')}
              </TabsTrigger>
              <TabsTrigger value='sources'>
                {t('Source management')}
              </TabsTrigger>
            </TabsList>
            <TabsContent value='sources'>
              <SourceManagement onReadDenied={handleReadDenied} />
            </TabsContent>
            <TabsContent value='preview'>
              {canManageSources ? (
                <div className='flex h-full min-h-0 flex-col gap-3'>
                  <SourceForm
                    value={source}
                    error={sourceError}
                    pending={previewMutation.isPending}
                    onChange={setSource}
                    onError={setSourceError}
                    onSubmit={submit}
                  />
                  {previewMutation.isPending ? (
                    <LoadingState
                      message={t('Downloading and parsing the subscription...')}
                    />
                  ) : null}
                  {previewMutation.isError ? (
                    <ErrorState
                      title={t('Subscription preview failed')}
                      description={getServerErrorMessage(previewMutation.error)}
                      onRetry={() =>
                        previewMutation.mutate(previewMutation.variables)
                      }
                      action={
                        previewErrorCode(previewMutation.error) ? (
                          <span className='text-muted-foreground font-mono text-xs'>
                            {previewErrorCode(previewMutation.error)}
                          </span>
                        ) : undefined
                      }
                    />
                  ) : null}
                  {previewMutation.isSuccess ? (
                    <PreviewResult
                      key={previewMutation.submittedAt}
                      preview={previewMutation.data}
                    />
                  ) : null}
                  {previewMutation.isIdle ? (
                    <EmptyState
                      icon={Rss}
                      title={t('No preview yet')}
                      description={t(
                        'Paste a subscription link to review parsing, transport capability, and per-node diagnostics.'
                      )}
                    />
                  ) : null}
                </div>
              ) : (
                <ErrorState
                  title={t(
                    'Source management permission is required to preview subscription links.'
                  )}
                />
              )}
            </TabsContent>
          </Tabs>
        ) : (
          <ErrorState title={t('Access to subscription preview has changed')} />
        )}
      </SectionPageLayout.Content>
    </SectionPageLayout>
  )
}
