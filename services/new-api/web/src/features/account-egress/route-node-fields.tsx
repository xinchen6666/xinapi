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
import { useId, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { ErrorState } from '@/components/error-state'
import { Button } from '@/components/ui/button'
import { Field, FieldDescription, FieldLabel } from '@/components/ui/field'
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

import type { RouteNode } from './console-api'

type RouteNodeFieldsProps = {
  userId: number
  sessionId: string
  selected: RouteNode | null
  onSelectionChange: (node: RouteNode | null) => void
  onPrepare: (node: RouteNode) => void
  busy: boolean
  mayPrepare: boolean
  continuation: boolean
}

export function RouteNodeFields(props: RouteNodeFieldsProps) {
  const { t } = useTranslation()
  const fieldId = useId()
  const [sourceId, setSourceId] = useState(props.selected?.source_id ?? '')
  const scope = [
    'account-egress',
    props.userId,
    'route-choices',
    props.sessionId,
  ]
  const sources = useQuery({
    queryKey: [...scope, 'sources'],
    queryFn: ({ signal }) => listSources(signal),
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const detail = useQuery({
    queryKey: [...scope, 'source', sourceId],
    queryFn: ({ signal }) => sourceDetail(sourceId, signal),
    enabled: !!sourceId,
    retry: false,
    gcTime: 0,
    meta: { errorToast: false },
  })
  const sourceItems = (sources.data ?? [])
    .filter((source) => source.approved_version > 0 && source.node_count > 0)
    .map((source) => ({ value: source.id, label: source.name }))
  const nodes = (detail.data?.nodes ?? []).filter(
    (node) => node.available && node.supported && !!node.version_id
  )
  const nodeItems = nodes.map((node) => ({ value: node.id, label: node.name }))
  const selected = props.selected
  const sourceAvailable = sourceItems.some((item) => item.value === sourceId)
  const selectedNode = nodes.find(
    (node) =>
      selected &&
      node.id === selected.node_id &&
      node.version_id === selected.version_id
  )
  const canPrepare =
    !!selected &&
    !!selectedNode &&
    sourceAvailable &&
    selected.source_id === sourceId &&
    props.mayPrepare &&
    !props.busy &&
    !sources.isFetching &&
    !detail.isFetching

  if (sources.isError || detail.isError) {
    return (
      <ErrorState
        title={t('Approved nodes are unavailable')}
        onRetry={() => {
          void sources.refetch()
          if (sourceId) void detail.refetch()
        }}
      />
    )
  }

  return (
    <div className='space-y-3'>
      <div className='grid min-w-0 gap-3 sm:grid-cols-2'>
        <Field className='min-w-0'>
          <FieldLabel htmlFor={`${fieldId}-source`}>
            {t('Approved source')}
          </FieldLabel>
          <Select
            items={sourceItems}
            value={sourceAvailable ? sourceId : ''}
            disabled={props.busy || sources.isPending}
            onValueChange={(value) => {
              setSourceId(value ?? '')
              props.onSelectionChange(null)
            }}
          >
            <SelectTrigger id={`${fieldId}-source`} className='w-full min-w-0'>
              <SelectValue placeholder={t('Select an approved source')} />
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
          {sources.isSuccess && sourceItems.length === 0 && (
            <FieldDescription>
              {t('Approve an available node in Subscription Preview first.')}
            </FieldDescription>
          )}
        </Field>
        <Field className='min-w-0'>
          <FieldLabel htmlFor={`${fieldId}-node`}>
            {t('Approved node')}
          </FieldLabel>
          <Select
            items={nodeItems}
            value={selectedNode?.id ?? ''}
            disabled={!sourceAvailable || props.busy || detail.isPending}
            onValueChange={(value) => {
              const node = nodes.find((item) => item.id === value)
              props.onSelectionChange(
                node
                  ? {
                      source_id: sourceId,
                      node_id: node.id,
                      version_id: node.version_id,
                      node_name: node.name,
                      protocol: node.protocol,
                    }
                  : null
              )
            }}
          >
            <SelectTrigger id={`${fieldId}-node`} className='w-full min-w-0'>
              <SelectValue placeholder={t('Select an approved node')} />
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
          {sourceAvailable && detail.isSuccess && nodeItems.length === 0 && (
            <FieldDescription>
              {t('This source has no approved node ready to run.')}
            </FieldDescription>
          )}
          {selected && !selectedNode && detail.isSuccess && (
            <FieldDescription>
              {t('The approved node version changed. Select the node again.')}
            </FieldDescription>
          )}
        </Field>
      </div>
      <Button
        size='sm'
        disabled={!canPrepare}
        onClick={() => {
          if (canPrepare && selected) props.onPrepare(selected)
        }}
      >
        {props.continuation
          ? t('Continue preparing route')
          : t('Prepare route')}
      </Button>
    </div>
  )
}
