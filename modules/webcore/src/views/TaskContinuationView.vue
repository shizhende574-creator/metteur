<script setup lang="ts">
import TaskPanel from '@/components/continuations/TaskPanel.vue'
import SchedulePanel from '@/components/continuations/SchedulePanel.vue'
import PageHeader from '@/components/PageHeader.vue'
import PageTabs from '@/components/PageTabs.vue'
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTabsStore } from '@/stores/tabs'
const route = useRoute(),
  router = useRouter()
const schedules = computed(() => route.query.view === 'schedules')
function select(view: string) {
  void router.replace({ query: view === 'schedules' ? { view } : {} })
}
useTabsStore().openSurface('tasks')
</script>
<template>
  <div class="surface-page">
    <PageHeader title="Tasks" />
    <PageTabs
      :model-value="schedules ? 'schedules' : 'tasks'"
      label="Tasks"
      :items="[
        { value: 'tasks', label: 'Tasks' },
        { value: 'schedules', label: 'Schedules' },
      ]"
      @update:model-value="select"
    />
    <SchedulePanel v-if="schedules" embedded />
    <TaskPanel v-else embedded />
  </div>
</template>
