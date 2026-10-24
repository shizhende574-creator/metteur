<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue'
import { gateway, type ChatMessage, type Blueprint } from '@/core'
import type { ExecutionSnapshot } from '@/core/execution-view'
import { useWorkspaceStore } from '@/stores/workspace'
import ChatThread from '@/components/chat/ChatThread.vue'
interface Activity {
  node_id: string
  scope: string
  frame: string[]
  invocation: number
  attempt: number
  messages: ChatMessage[]
  truncated: boolean
}
interface Output {
  run_id: string
  revision: number
  nodes: Activity[]
  truncated: boolean
}
const props = defineProps<{
  runId: string | null
  connected: boolean
  snapshot?: ExecutionSnapshot
  blueprint?: Blueprint | null
  revision?: number
}>()
const emit = defineEmits<{ file: [path: string] }>()
const workspace = useWorkspaceStore(),
  output = ref<Output | null>(null),
  error = ref('')
let generation = 0,
  timer: ReturnType<typeof setTimeout> | undefined,
  busy = false,
  again = false
async function load() {
  const root = workspace.active?.path,
    run = props.runId,
    ticket = generation
  if (!root || !run || !props.connected) return
  if (busy) {
    again = true
    return
  }
  busy = true
  try {
    const result = await gateway.resourceRequest('queryCodingWorkflows', root, {
      operation: 'output',
      run_id: run,
    })
    if (ticket !== generation || root !== workspace.active?.path || run !== props.runId) return
    if (!result.ok) {
      error.value = 'Recorded node output is unavailable. Reconnect or refresh this run.'
      return
    }
    const value = result.data as Output
    if (value.run_id !== run || !Array.isArray(value.nodes))
      throw new Error('Output identity mismatch')
    if (!output.value || value.revision >= output.value.revision) output.value = value
    error.value = ''
  } catch (cause) {
    if (ticket === generation) error.value = String(cause)
  } finally {
    busy = false
    if (again) {
      again = false
      schedule()
    }
  }
}
function schedule() {
  clearTimeout(timer)
  timer = setTimeout(() => void load(), 150)
}
watch(
  [() => props.runId, () => workspace.active?.path],
  () => {
    generation++
    output.value = null
    error.value = ''
    schedule()
  },
  { immediate: true },
)
watch([() => props.connected, () => props.revision, () => props.snapshot?.view?.sequence], schedule)
onBeforeUnmount(() => {
  generation++
  clearTimeout(timer)
})
const messages = computed<ChatMessage[]>(() => {
  const rows: ChatMessage[] = []
  const invocations = [...(props.snapshot?.view?.invocations ?? [])]
  for (const activity of output.value?.nodes ?? []) {
    if (
      !invocations.some(
        (i) =>
          i.sequence === activity.invocation &&
          i.node_id === activity.node_id &&
          i.scope === activity.scope &&
          JSON.stringify(i.frame) === JSON.stringify(activity.frame),
      )
    )
      invocations.push({
        sequence: activity.invocation,
        node_id: activity.node_id,
        scope: activity.scope,
        frame: activity.frame,
        attempt: activity.attempt,
        inputs: {},
        outputs: {},
        started_at: activity.messages[0]?.createdAt ?? 0,
        status: 'Recorded activity',
        messages: [],
      })
  }
  invocations.sort((a, b) => a.sequence - b.sequence)
  for (const invocation of invocations) {
    const graph =
      props.snapshot?.view?.graphs[invocation.scope] ??
      props.snapshot?.view?.root ??
      props.blueprint
    const node = graph?.nodes.find((n) => n.id === invocation.node_id),
      name = node?.title ?? invocation.node_id
    const activity = output.value?.nodes.find(
      (n) =>
        n.invocation === invocation.sequence &&
        n.node_id === invocation.node_id &&
        n.scope === invocation.scope &&
        JSON.stringify(n.frame) === JSON.stringify(invocation.frame),
    )
    const outputs = Object.fromEntries(
      Object.entries(invocation.outputs ?? {}).flatMap(([key, value]) => {
        const pin = node?.outputs.find((p) => p.id === key)
        return pin?.kind === 'exec-out' || pin?.name === 'Context'
          ? []
          : [[pin?.name ?? key, value]]
      }),
    )
    if (!activity?.messages.length && !Object.keys(outputs).length && !invocation.messages.length)
      continue
    const prefix = `${props.runId}-${invocation.sequence}`
    rows.push({
      id: prefix + '-heading',
      role: 'notice',
      content: `${name} · Attempt ${invocation.attempt} · ${invocation.status}${invocation.current === false ? ' · Previous attempt' : ''}${invocation.frame.length ? ' · ' + invocation.frame.join(' / ') : ''}`,
      createdAt: invocation.started_at,
    })
    if (activity?.messages.length)
      rows.push(
        ...activity.messages.map((m) => ({
          ...m,
          id: prefix + '-' + m.id,
          pending: !!m.pending && invocation.finished_at == null && props.connected,
        })),
      )
    else if (node?.type === 'CallLLM' && typeof outputs.Result === 'string')
      rows.push({
        id: prefix + '-result',
        role: 'assistant',
        content: outputs.Result,
        createdAt: invocation.finished_at ?? invocation.started_at,
      })
    else if (Object.keys(outputs).length)
      rows.push({
        id: prefix + '-outputs',
        role: 'tool',
        actor: node?.type ?? name,
        content: JSON.stringify(outputs, null, 2),
        createdAt: invocation.started_at,
        detail: { ok: invocation.status === 'Completed', summary: 'Recorded node output' },
      })
    for (const [i, text] of invocation.messages.entries())
      rows.push({
        id: prefix + `-note${i}`,
        role: 'notice',
        content: text,
        createdAt: invocation.finished_at ?? invocation.started_at,
      })
    if (activity?.truncated)
      rows.push({
        id: prefix + '-limit',
        role: 'notice',
        content: 'Output limit reached · some node text was truncated.',
        createdAt: invocation.started_at,
      })
  }
  return rows
})
const noop = () => {}
function copy(text: string) {
  void navigator.clipboard.writeText(text)
}
</script>
<template>
  <section class="execution-output" aria-label="Node output">
    <p v-if="!connected" role="status" class="output-notice">
      Disconnected · showing the last recorded output.
    </p>
    <p v-if="error" role="alert" class="output-notice">
      {{ error }} <button class="btn" @click="load">Refresh output</button>
    </p>
    <p v-if="output?.truncated" role="status" class="output-notice">
      Run output limit reached. Earlier records are retained; inspect node details for committed
      results.
    </p>
    <ChatThread
      v-if="runId"
      :messages="messages"
      read-only
      :on-open-file="(path) => emit('file', path)"
      :on-copy="copy"
      :on-edit="noop"
      :on-retry="noop"
      :on-restore="noop"
      ><template #empty
        ><p class="output-notice">No recorded node output yet.</p></template
      ></ChatThread
    >
    <p v-else class="output-notice">Run a saved blueprint to see its node output here.</p>
  </section>
</template>
<style scoped>
.execution-output {
  display: flex;
  flex: 1;
  min-height: 0;
  flex-direction: column;
  overflow: hidden;
  font-size: 13px;
}
.execution-output :deep(.chat-column) {
  width: 100%;
  max-width: none;
  padding: 18px;
}
.execution-output :deep(.md-body) {
  font-size: 14px;
  line-height: 1.75;
}
.execution-output :deep(.chat-notice) {
  font-size: 12px;
  font-weight: 500;
  text-align: left;
  border-top: 1px solid var(--divider);
  padding-top: 14px;
  margin: 20px 0 12px;
}
.output-notice {
  padding: 12px 18px;
  color: var(--muted-foreground);
  line-height: 1.6;
  font-size: 13px;
}
.output-notice[role='alert'] {
  color: var(--danger);
}
</style>
