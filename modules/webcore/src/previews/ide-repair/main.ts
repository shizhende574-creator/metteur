import { createApp } from 'vue'
import { createPinia } from 'pinia'
import '@/styles/main.css'
import App from '@/App.vue'
import { router } from '@/router'
import { useWorkspaceStore } from '@/stores/workspace'
import { useTabsStore } from '@/stores/tabs'
import Preview from './Preview.vue'
if (import.meta.env.VITE_MOCK !== '1') throw new Error('Usability preview requires fixture gateway')
router.addRoute({
  path: '/src/previews/ide-repair/index.html',
  name: 'ide-usability-preview',
  component: Preview,
  meta: { title: 'IDE preview · r1' },
})
const app = createApp(App),
  pinia = createPinia()
app.use(pinia)
await useWorkspaceStore(pinia).open('D:/workflow-fixture/metrics')
useTabsStore(pinia).openSurface('chat')
app.use(router)
await router.isReady()
app.mount('#app')
