<script setup lang="ts">
import MemoryPanel from '@/components/resources/MemoryPanel.vue'
import { useTabsStore } from '@/stores/tabs'
useTabsStore().openSurface('memory')
</script>
<template>
  <div class="surface-page memory-page"><MemoryPanel /></div>
</template>
<style scoped>
.memory-page {
  height: 100%;
  min-height: 0;
}
</style>
