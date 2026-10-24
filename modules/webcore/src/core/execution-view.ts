import type { Blueprint, BlueprintPin } from './types'
import { categoryFor } from '@/lib/blueprint'
import { inlineValue, pinType, valueText } from './node-catalog'
export interface VersionRef {
  snapshot_id: string
  blueprint_uri: string
  blob_hash: string
}
export interface Invocation {
  sequence: number
  current?: boolean
  node_id: string
  scope: string
  frame: string[]
  attempt: number
  version?: VersionRef
  inputs: Record<string, unknown>
  outputs: Record<string, unknown>
  started_at: number
  finished_at?: number
  status: string
  messages: string[]
}
export interface Traversal {
  edge_id: string
  scope: string
  frame: string[]
  sequence: number
}
export interface ExecutionSnapshot {
  blueprint_version?: VersionRef
  in_flight?: string
  executed: string[]
  pending: string[]
  error?: string
  runtime?: {
    active: boolean
    pause_requested: boolean
    cancel_requested: boolean
    pending_approval_ids: string[]
    pending_approvals?: { request_id: string; detail: string; content_digest?: string }[]
  }
  view?: {
    root?: Blueprint
    graphs: Record<string, Blueprint>
    invocations: Invocation[]
    edges: Traversal[]
    sequence: number
  }
}
export interface NativeGraph {
  explicit_null_defaults?: string[]
  id: string
  name: string
  entry_node_id: string
  nodes: {
    id: string
    kind: string
    node_type: string
    position: [number, number]
    data: Record<string, unknown>
    pins: {
      id: string
      name: string
      key?: string
      pin_type: string
      data_type: string
      optional?: boolean
      default?: unknown
      choices?: string[]
      description?: string
    }[]
  }[]
  edges: {
    id: string
    source_node: string
    source_pin: string
    target_node: string
    target_pin: string
  }[]
}
export function nativeBlueprint(raw: NativeGraph): Blueprint {
  return {
    id: raw.id,
    name: raw.name,
    entryNodeId: raw.entry_node_id,
    nodes: raw.nodes.map((n) => {
      const pins: BlueprintPin[] = n.pins.map((p) => ({
        id: p.id,
        name: p.name,
        key: p.key,
        kind: (
          {
            ExecInput: 'exec-in',
            ExecOutput: 'exec-out',
            DataInput: 'data-in',
            DataOutput: 'data-out',
          } as Record<string, BlueprintPin['kind']>
        )[p.pin_type]!,
        type: pinType(p.data_type),
        optional: p.optional,
        default:
          p.default === null && !raw.explicit_null_defaults?.includes(p.id) ? undefined : p.default,
        choices: p.choices,
        description: p.description,
      }))
      return {
        id: n.id,
        title: n.kind,
        type: n.kind,
        nodeType: n.node_type,
        position: { x: n.position[0], y: n.position[1] },
        data: n.data,
        values: Object.fromEntries(
          pins
            .filter((p) => p.kind === 'data-in')
            .flatMap((p) => {
              const value = inlineValue(n.data, p)
              return value === undefined ? [] : [[p.id, valueText(value)]]
            }),
        ),
        category: categoryFor(n.kind, n.node_type),
        inputs: pins.filter((p) => p.kind.endsWith('-in')),
        outputs: pins.filter((p) => p.kind.endsWith('-out')),
      }
    }),
    edges: raw.edges.map((e) => ({
      id: e.id,
      source: e.source_node,
      sourceHandle: e.source_pin,
      target: e.target_node,
      targetHandle: e.target_pin,
    })),
  }
}
/** Missing legacy presentation metadata is unavailable, never inferred from node order. */
export function executionSnapshot(json: string): ExecutionSnapshot | undefined {
  try {
    const raw = JSON.parse(json)
    if (!raw || !Array.isArray(raw.executed) || !Array.isArray(raw.pending)) return undefined
    if (raw.view?.root) raw.view.root = nativeBlueprint(raw.view.root)
    if (raw.view?.graphs)
      raw.view.graphs = Object.fromEntries(
        Object.entries(raw.view.graphs).map(([id, g]) => [id, nativeBlueprint(g as NativeGraph)]),
      )
    return raw
  } catch {
    return undefined
  }
}
