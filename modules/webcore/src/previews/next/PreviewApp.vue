<script setup lang="ts">
import registry from './registry.json'
</script>
<template>
  <main class="p-8 max-w-5xl mx-auto">
    <p class="text-muted-foreground">METTEUR / ISOLATED PREVIEWS</p>
    <h1 class="text-2xl font-semibold my-3">NX preview registry</h1>
    <p>Fixture data only. No daemon, model API or user data is connected.</p>
    <p class="my-3">Fixture: {{ registry.fixtureVersion }} · User approval pending</p>
    <ul class="space-y-3"><li v-for="preview in registry.previews" :key="preview.id">{{ preview.id }} · {{ preview.title }} · {{ preview.state }}</li></ul>
  </main>
</template>
