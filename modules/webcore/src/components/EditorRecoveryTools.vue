<script setup lang="ts">
import { History, Ellipsis } from '@lucide/vue'
defineProps<{ label: string; state?: string; error?: string; dirty: boolean }>()
const emit = defineEmits<{ open: []; backup: []; discard: [] }>()
</script>
<template>
  <span role="status" class="sr-only">{{ label }}</span>
  <button
    type="button"
    class="editor-tool-icon"
    :class="state && !['backed_up', 'backing_up'].includes(state) ? 'text-status-warning' : ''"
    :title="`Recovery Copies · ${label}`"
    aria-label="Recovery Copies"
    @click="emit('open')"
  >
    <History class="h-4 w-4" />
  </button>
  <details v-if="dirty || error" class="relative recovery-draft-options">
    <summary
      class="editor-tool-icon list-none cursor-pointer"
      title="Draft options"
      aria-label="Draft Options"
    >
      <Ellipsis class="h-4 w-4" />
    </summary>
    <div
      class="absolute right-0 top-full z-50 mt-2 w-72 rounded-lg border border-border bg-popover p-3 text-[13px] shadow-popover"
    >
      <p class="mb-3 break-words" :class="error ? 'text-status-warning' : 'text-muted-foreground'">
        {{ error || label }}
      </p>
      <div class="flex gap-2">
        <button
          v-if="state && !['backed_up', 'backing_up'].includes(state)"
          class="btn"
          @click="emit('backup')"
        >
          Back Up</button
        ><button v-if="dirty" class="btn" @click="emit('discard')">Discard Draft</button>
      </div>
    </div>
  </details>
</template>
