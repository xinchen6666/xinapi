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
import type { ColumnDef } from '@tanstack/react-table'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { DataTablePage, useDataTable } from '@/components/data-table'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'

import type { SourceCandidate, SourceNode } from '../source-types'

export type CandidateDecision = { selected: boolean; target: string }
export type CandidateDecisions = Record<number, CandidateDecision>
type Props = {
  candidates: SourceCandidate[]
  nodes: SourceNode[]
  decisions: CandidateDecisions
  canWrite: boolean
  onChange: (index: number, value: CandidateDecision) => void
}

export function ReviewCandidates(props: Props) {
  const { t } = useTranslation()
  const columns = useMemo<ColumnDef<SourceCandidate, unknown>[]>(
    () => [
      {
        id: 'select',
        header: t('Select'),
        size: 60,
        cell: ({ row }) => {
          const decision = props.decisions[row.original.index] ?? {
            selected: false,
            target: '',
          }
          return (
            <Checkbox
              checked={decision.selected}
              disabled={!props.canWrite || !row.original.parse_valid}
              aria-label={t('Select {{name}}', { name: row.original.name })}
              onCheckedChange={(value) =>
                props.onChange(row.original.index, {
                  ...decision,
                  selected: Boolean(value),
                })
              }
            />
          )
        },
      },
      {
        accessorKey: 'name',
        header: t('Name'),
        meta: { mobileTitle: true },
        cell: ({ row }) => (
          <span className='break-all'>{row.original.name}</span>
        ),
      },
      { accessorKey: 'protocol', header: t('Protocol'), size: 100 },
      {
        accessorKey: 'change',
        header: t('Refresh difference'),
        cell: ({ row }) => {
          const labels = {
            unchanged: t('Unchanged'),
            renamed: t('Renamed'),
            added: t('Added'),
            connection_changed: t('Connection changed'),
            ambiguous: t('Identity decision required'),
            invalid: t('Needs correction'),
          }
          return labels[row.original.change] ?? t('Identity decision required')
        },
      },
      {
        accessorKey: 'supported',
        header: t('Transport'),
        cell: ({ row }) =>
          row.original.supported ? t('Native HTTP/SOCKS') : t('Mihomo pending'),
      },
      {
        id: 'identity',
        header: t('Node identity'),
        size: 300,
        cell: ({ row }) => {
          const decision = props.decisions[row.original.index] ?? {
            selected: false,
            target: '',
          }
          const selectedNode = props.nodes.find(
            (node) => node.id === decision.target
          )
          let label = t('Choose node identity')
          if (decision.target === 'new') label = t('Create new node identity')
          if (selectedNode) label = `${selectedNode.name} · ${selectedNode.id}`
          return (
            <div className='space-y-1'>
              <Select
                value={decision.target || null}
                disabled={!props.canWrite || !row.original.parse_valid}
                onValueChange={(value) =>
                  props.onChange(row.original.index, {
                    ...decision,
                    target: typeof value === 'string' ? value : '',
                  })
                }
              >
                <SelectTrigger
                  className='w-full max-w-80'
                  aria-label={t('Node identity for {{name}}', {
                    name: row.original.name,
                  })}
                >
                  <SelectValue>{label}</SelectValue>
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value='new'>
                    {t('Create new node identity')}
                  </SelectItem>
                  {props.nodes
                    .filter((node) =>
                      row.original.previous_node_ids.includes(node.id)
                    )
                    .map((node) => (
                      <SelectItem key={node.id} value={node.id}>
                        {node.name} · {node.id}
                      </SelectItem>
                    ))}
                </SelectContent>
              </Select>
              {row.original.suggested_node_id ? (
                <span className='text-muted-foreground block text-xs break-all'>
                  {t('Suggested identity')}: {row.original.suggested_node_id}
                </span>
              ) : null}
              {row.original.previous_node_ids.length ? (
                <span className='text-muted-foreground block text-xs break-all'>
                  {t('Previous identities')}:{' '}
                  {row.original.previous_node_ids.join(', ')}
                </span>
              ) : null}
            </div>
          )
        },
      },
    ],
    [t, props]
  )
  const { table } = useDataTable({
    data: props.candidates,
    columns,
    getRowId: (row) => String(row.index),
    enableSorting: false,
    enableRowSelection: false,
  })
  return (
    <DataTablePage
      table={table}
      columns={columns}
      fixedHeight={false}
      emptyTitle={t('The preview found no nodes in this subscription')}
      toolbar={
        props.canWrite ? (
          <Button
            variant='outline'
            onClick={() => {
              for (const candidate of props.candidates) {
                const current = props.decisions[candidate.index] ?? {
                  selected: false,
                  target: '',
                }
                props.onChange(candidate.index, {
                  ...current,
                  selected: candidate.parse_valid,
                })
              }
            }}
          >
            {t('Select valid candidates')}
          </Button>
        ) : undefined
      }
    />
  )
}
