<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'
import { FileCode2, Play } from '@lucide/vue'
import { gateway, type Blueprint } from '@/core'
import { useExecutionStore } from '@/stores/execution'
import { useWorkspaceStore } from '@/stores/workspace'
import { useTabsStore } from '@/stores/tabs'
import { usePanelStore } from '@/stores/panel'
import { fileRoute } from '@/lib/file-token'
import ExecutionLayout from '@/components/execution/ExecutionLayout.vue'
import FilePickerDialog, { type FilePick } from '@/components/FilePickerDialog.vue'
const execution = useExecutionStore(), workspace = useWorkspaceStore(), tabs = useTabsStore()
const router = useRouter(), panel = usePanelStore()
const picker = ref(false), selected = ref<Blueprint | null>(null), path = ref(''), error = ref(''), busy = ref(false)
const graph = computed(() => execution.blueprint ?? selected.value)
const source = computed(() => execution.sourcePath || path.value)
onMounted(() => { tabs.openSurface('execution'); panel.clear(); void execution.reconcile() })
async function pick(file: FilePick) {
  picker.value = false
  const ws = workspace.active?.path
  if (!ws || !file.filePath || execution.running) return
  busy.value = true; error.value = ''
  try {
    const read = await gateway.readFile(ws, file.filePath)
    if (!read.ok) throw new Error(read.error)
    const parsed = JSON.parse(read.data.content)
    if (!parsed.id) throw new Error('Save this blueprint in the editor before running.')
    const loaded = await gateway.loadBlueprint(ws, parsed.id)
    if (!loaded.ok) throw new Error(loaded.error)
    if (workspace.active?.path !== ws) return
    selected.value = loaded.data; path.value = file.filePath
    execution.blueprint = null; execution.sourcePath = ''
  } catch (e) { error.value = String(e) } finally { busy.value = false }
}
function run() { if (graph.value) void execution.run(graph.value.id, graph.value, source.value) }
function openSource() { if (source.value) { tabs.openFile(source.value); void router.push(fileRoute(source.value)) } }
</script>
<template>
  <ExecutionLayout>
    <template #header>
      <button class="blueprint-select" :disabled="execution.running || execution.launching || busy" @click="picker = true">{{ graph?.name || 'Select blueprint' }}</button>
      <span class="status">{{ execution.status === 'idle' ? 'Ready' : execution.status }}</span>
      <button class="action primary" :disabled="!graph || execution.running || execution.launching || busy" @click="run"><Play :size="13" />Run</button>
      <span v-if="error || execution.error" role="alert">{{ error || execution.error }}</span>
    </template>
    <template #tools><button aria-label="Open blueprint source" :disabled="!source" @click="openSource"><FileCode2 :size="15" /></button></template>
    <template #graph><div class="empty">{{ graph ? 'Execution details will appear here.' : 'Select a saved blueprint to run.' }}</div></template>
    <template #details="{ tab }">
      <div v-if="tab === 'Execution log'" class="details"><p v-for="(event, i) in execution.events" :key="i">{{ event.kind }} · {{ event.message }}</p></div>
      <div v-else class="details">{{ tab === 'Node details' ? 'Select a node to inspect its execution.' : `${tab} is not connected yet.` }}</div>
    </template>
  </ExecutionLayout>
  <FilePickerDialog :open="picker" mode="open" title="Select blueprint" :workspace-path="workspace.active?.path ?? ''" :extensions="['.blueprint']" @close="picker = false" @confirm="pick" />
</template>
<style scoped>
.blueprint-select{font-size:13px;max-width:300px;overflow:hidden;text-overflow:ellipsis}.status{color:var(--muted-foreground);font-size:12px}.action{display:flex;align-items:center;gap:6px;padding:7px 10px;border:1px solid var(--border);border-radius:6px;margin-left:auto}.primary{background:var(--primary);color:white}button:disabled{opacity:.45}.details{padding:20px;color:var(--muted-foreground);overflow-wrap:anywhere}.empty{height:100%;display:grid;place-items:center;color:var(--muted-foreground)}[role=alert]{color:var(--danger);width:100%}
</style>
