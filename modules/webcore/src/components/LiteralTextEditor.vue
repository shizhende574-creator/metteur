<script setup lang="ts">
import { ref, watch } from 'vue'
import { Maximize2, X } from '@lucide/vue'
import { vDialogFocus } from './resources/dialog-focus'
const value = defineModel<string>({ required: true })
const props = defineProps<{ label: string; placeholder?: string; disabled?: boolean }>()
const emit = defineEmits<{ save: [] }>()
const expanded = ref(false),
  draft = ref('')
watch(
  () => props.label,
  () => {
    expanded.value = false
  },
)
function open() {
  draft.value = value.value
  expanded.value = true
}
function apply() {
  value.value = draft.value
  expanded.value = false
}
function expandedKey(e: KeyboardEvent) {
  if (e.isComposing) return
  if (e.key === 'Escape') {
    e.stopPropagation()
    e.preventDefault()
    expanded.value = false
  }
  if (e.ctrlKey || e.metaKey) {
    if (e.key === 'Enter') {
      e.preventDefault()
      apply()
    }
    if (e.key.toLowerCase() === 's') {
      e.preventDefault()
      value.value = draft.value
      emit('save')
    }
  }
}
</script>
<template>
  <div class="literal-text-field">
    <input
      v-model="value"
      :aria-label="label"
      :placeholder="placeholder"
      :disabled="disabled"
      :readonly="/[\r\n]/.test(value)"
      :title="/[\r\n]/.test(value) ? 'Expand to edit multi-line text' : undefined"
      @dblclick="open"
      class="input"
    />
    <button
      type="button"
      :disabled="disabled"
      :aria-label="`Expand ${label}`"
      title="Expand text"
      @click="open"
    >
      <Maximize2 :size="14" />
    </button>
  </div>
  <Teleport to="body">
    <div v-if="expanded" v-dialog-focus class="literal-editor-overlay" @keydown="expandedKey">
      <section
        role="dialog"
        aria-modal="true"
        :aria-label="`Edit ${label}`"
        class="literal-editor-dialog"
      >
        <header>
          <strong>{{ label }}</strong
          ><button type="button" aria-label="Close text editor" @click="expanded = false">
            <X :size="17" />
          </button>
        </header>
        <textarea
          v-model="draft"
          :aria-label="`${label} expanded`"
          wrap="soft"
          spellcheck="false"
          autofocus
        />
        <footer>
          <span>Soft wrap · Ctrl+Enter to apply</span
          ><button class="btn" @click="expanded = false">Cancel</button
          ><button class="btn btn-primary" @click="apply">Apply</button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>
<style scoped>
.literal-text-field {
  display: flex;
  align-items: center;
  position: relative;
  min-width: 0;
}
.literal-text-field input {
  width: 100%;
  padding-right: 31px;
}
.literal-text-field > button {
  position: absolute;
  right: 4px;
  display: grid;
  place-items: center;
  width: 25px;
  height: 25px;
  color: var(--muted-foreground);
  border-radius: 4px;
}
.literal-text-field > button:hover {
  background: var(--hover);
}
.literal-editor-overlay {
  position: fixed;
  inset: 0;
  z-index: 130;
  background: #0006;
  display: grid;
  place-items: center;
  padding: 20px;
}
.literal-editor-dialog {
  width: min(760px, 100%);
  display: flex;
  flex-direction: column;
  max-height: 85vh;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--popover);
  box-shadow: var(--shadow-popover);
  font-size: 14px;
}
.literal-editor-dialog header,
.literal-editor-dialog footer {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 14px 18px;
}
.literal-editor-dialog header {
  border-bottom: 1px solid var(--divider);
}
.literal-editor-dialog header button {
  margin-left: auto;
}
.literal-editor-dialog textarea {
  resize: vertical;
  min-height: 260px;
  max-height: 60vh;
  padding: 16px 18px;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  outline: none;
  font-family: var(--font-mono, monospace);
  line-height: 1.65;
}
.literal-editor-dialog footer {
  border-top: 1px solid var(--divider);
}
.literal-editor-dialog footer span {
  margin-right: auto;
  color: var(--muted-foreground);
  font-size: 12px;
}
</style>
