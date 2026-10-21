<script setup lang="ts">
defineProps<{ title: string; description?: string }>()
</script>

<template>
  <header class="surface-header">
    <div class="min-w-0">
      <div class="surface-heading">
        <h1 class="surface-title">{{ title }}</h1>
        <slot name="help" />
      </div>
      <p v-if="description" class="surface-description">{{ description }}</p>
    </div>
    <div v-if="$slots.default" class="surface-actions"><slot /></div>
  </header>
</template>
