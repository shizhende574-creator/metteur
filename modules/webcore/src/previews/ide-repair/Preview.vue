<script setup lang="ts">
import { ref } from 'vue'
import {
  History,
  LayoutTemplate,
  Save,
  Undo2,
  Redo2,
  GitBranch,
  Play,
  Square,
  ShieldCheck,
  ChevronRight,
  Brain,
  SunMoon,
  X,
} from '@lucide/vue'
import PageHeader from '@/components/PageHeader.vue'
import LiteralTextEditor from '@/components/LiteralTextEditor.vue'
import { useThemeStore } from '@/stores/theme'
const group = ref(
  new URLSearchParams(location.search).get('preview') === 'IU-PV-02' ? 'IU-PV-02' : 'IU-PV-01',
)
const screen = ref(group.value === 'IU-PV-02' ? 'Execution' : 'Editor'),
  tab = ref('Output'),
  node = ref('CallLLM'),
  graph = ref('fix-normalize.blueprint')
const text = ref(
    'Implement the normalization fix. Preserve the public API and Chinese messages. Add a regression test for blank input. Run the approved checks after the scoped change.',
  ),
  enabled = ref(false)
const steps = ref(false),
  recovery = ref(false),
  selected = ref(0),
  notice = ref(''),
  state = ref('Finished'),
  saved = ref(text.value)
const theme = useThemeStore()
function save() {
  saved.value = text.value
  notice.value = 'Fixture draft saved'
}
function changeScreen(value: string) {
  screen.value = value
  notice.value = ''
}
</script>
<template>
  <div class="iu-preview">
    <div class="preview-controls">
      <span>Isolated fixture · {{ group }} / r1 · No real execution</span
      ><select
        v-model="group"
        aria-label="Preview group"
        @change="changeScreen(group === 'IU-PV-02' ? 'Execution' : 'Editor')"
      >
        <option>IU-PV-01</option>
        <option>IU-PV-02</option></select
      ><button
        v-for="s in ['Editor', 'Chat', 'Execution', 'Memory']"
        :key="s"
        :class="{ active: screen === s }"
        @click="changeScreen(s)"
      >
        {{ s }}</button
      ><button aria-label="Toggle theme" @click="theme.toggle()"><SunMoon :size="15" /></button>
    </div>
    <template v-if="screen === 'Editor'">
      <div class="editor-toolbar" role="toolbar" aria-label="Blueprint editor tools">
        <select v-model="graph" aria-label="Blueprint file">
          <option>fix-normalize.blueprint</option>
          <option>new-feature.blueprint</option></select
        ><span v-if="graph === 'fix-normalize.blueprint'" class="muted"
          >Last run · {{ state.toLowerCase() }}</span
        ><span class="spacer" /><button
          title="Start from template"
          aria-label="Start from template"
          @click="notice = 'Template catalog opens here'"
        >
          <LayoutTemplate :size="16" /></button
        ><button
          title="Recovery copies · Backed up on host"
          aria-label="Recovery copies"
          @click="recovery = true"
        >
          <History :size="16" /></button
        ><button title="Undo" aria-label="Undo" @click="text = saved"><Undo2 :size="16" /></button
        ><button title="Redo" aria-label="Redo"><Redo2 :size="16" /></button
        ><button title="Version history" aria-label="Version history">
          <GitBranch :size="16" /></button
        ><button title="Save (Ctrl+S)" aria-label="Save" @click="save"><Save :size="16" /></button
        ><button
          title="Run"
          aria-label="Run"
          @click="notice = 'Fixture: exact version confirmation would open'"
        >
          <Play :size="16" />
        </button>
      </div>
      <div
        class="canvas"
        tabindex="0"
        @keydown="
          (e) => {
            if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 's') {
              e.preventDefault()
              save()
            }
          }
        "
      >
        <div class="preview-node" @click="node = 'CallLLM'">
          <header>CallLLM</header>
          <div class="pin-row"><i class="exec" /><span> </span></div>
          <label
            ><span>Prompt <small>string</small></span
            ><input v-model="text" aria-label="Node prompt" @keydown.ctrl.s.prevent="save" /></label
          ><label
            ><span>Temperature <small>float</small></span
            ><input value="0.2" aria-label="Temperature" /></label
          ><label
            ><span>Enabled <small>bool</small></span
            ><input v-model="enabled" type="checkbox" aria-label="Enabled"
          /></label>
          <div class="pin-output">
            <span>Result <small>string</small></span
            ><i />
          </div>
        </div>
        <div class="preview-wire" />
        <div class="preview-node validator" @click="node = 'Validator'">
          <header>Validator</header>
          <div class="pin-row"><i class="exec" /></div>
          <label
            ><span>Expected <small>int</small></span
            ><input value="0" aria-label="Expected"
          /></label>
          <div class="pin-output">
            <span>Passed <small>bool</small></span
            ><i />
          </div>
        </div>
        <aside class="inspector">
          <header>
            {{ node }}<span class="spacer" /><button
              aria-label="Close inspector"
              @click="notice = 'Inspector closed in production'"
            >
              <X :size="14" />
            </button>
          </header>
          <label
            ><span>Prompt <small>string</small></span
            ><LiteralTextEditor v-model="text" label="Prompt" /></label
          ><label
            ><span>Enabled <small>bool</small></span
            ><input v-model="enabled" type="checkbox" aria-label="Inspector enabled"
          /></label>
          <p class="muted">Literal inputs · edit the saved blueprint</p>
        </aside>
      </div>
    </template>
    <template v-else-if="screen === 'Chat'">
      <PageHeader title="Chat" toolbar
        ><button
          class="icon"
          title="Start from template"
          aria-label="Start from template"
          @click="notice = 'Template catalog opens here'"
        >
          <LayoutTemplate :size="16" /></button
      ></PageHeader>
      <div class="chat-column chat-example">
        <p class="user-text">Fix the normalization bug and verify the change.</p>
        <button class="step-summary" :aria-expanded="steps" @click="steps = !steps">
          <ChevronRight :size="14" :class="{ rotated: steps }" /><span>4 steps · 8.2s</span>
        </button>
        <div v-if="steps" class="step-body">
          <p>
            <Brain :size="14" />Thought · Inspect the shared function and preserve its public API.
          </p>
          <details>
            <summary>Read shared.py</summary>
            <pre>def normalize(value): return value.strip()</pre>
          </details>
          <details>
            <summary>Edit shared.py</summary>
            <p>Changed blank input handling.</p>
          </details>
          <details>
            <summary>Run regression checks · passed</summary>
            <pre>Ran 4 tests · OK</pre>
          </details>
        </div>
        <div class="answer">
          <p>
            Fixed normalization for blank input and preserved both callers. All four regression
            checks passed.
          </p>
          <p class="muted">
            Final answer stays visible. Reasoning and tools share the steps group.
          </p>
        </div>
      </div>
      <div class="composer">
        <textarea aria-label="Message" placeholder="Continue this conversation…" />
        <div class="muted">
          Backed up on host
          <button class="icon" aria-label="Chat recovery copies" @click="recovery = true">
            <History :size="14" />
          </button>
        </div>
      </div>
    </template>
    <template v-else-if="screen === 'Execution'">
      <PageHeader title="fix-normalize" toolbar
        ><select v-model="state" aria-label="Run state">
          <option>Finished</option>
          <option>Stopped by you</option>
          <option>Failed</option>
          <option>Running</option></select
        ><span class="run-state" :class="{ failure: state === 'Failed' }">{{ state }}</span
        ><button class="icon" aria-label="Stop fixture run" @click="state = 'Stopped by you'">
          <Square :size="14" /></button
      ></PageHeader>
      <div class="execution-body">
        <div class="execution-main">
          <div class="canvas">
            <div class="preview-node runtime">
              <header>CallLLM</header>
              <label
                ><span>Prompt <small>string</small></span>
                <p class="literal">{{ text }}</p></label
              >
              <div class="pin-output">
                <span>Result <small>string</small></span
                ><i />
              </div>
            </div>
          </div>
          <div class="bottom-detail">
            <strong>Node details</strong>
            <p class="muted">Configured literal · Prompt</p>
            <p>{{ text }}</p>
            <details>
              <summary>Raw run diagnostics</summary>
              <pre>{{
                state === 'Stopped by you' ? 'execution interrupted: cancelled by user' : 'No error'
              }}</pre>
            </details>
          </div>
        </div>
        <aside class="supervision">
          <header><ShieldCheck :size="16" /><strong>Supervision</strong></header>
          <nav>
            <button
              v-for="t in ['Output', 'Chat', 'Requests', 'Reviews']"
              :key="t"
              :class="{ active: tab === t }"
              @click="tab = t"
            >
              {{ t }}
            </button>
          </nav>
          <div v-if="tab === 'Output'" class="output">
            <div class="output-label">CallLLM <span>Attempt 1 · Finished</span></div>
            <button class="step-summary" :aria-expanded="steps" @click="steps = !steps">
              <ChevronRight :size="13" :class="{ rotated: steps }" />3 steps · 4.1s
            </button>
            <div v-if="steps" class="step-body">
              <p><Brain :size="13" />Thought · Trace blank input behavior.</p>
              <details>
                <summary>Read shared.py</summary>
                <pre>Recorded tool result</pre>
              </details>
              <details>
                <summary>Edit shared.py</summary>
                <p>Scoped edit completed.</p>
              </details>
            </div>
            <p>Implemented the blank input fix. The approved checks run in the next node.</p>
            <div class="output-label">ExecuteCommand <span>Attempt 1 · Finished</span></div>
            <pre>
Ran 4 tests in 0.004s
OK</pre>
            <p v-if="state === 'Stopped by you'" class="muted">
              Stopped by you · Partial output retained.
            </p>
            <p v-if="state === 'Failed'" class="failure">
              Check failed. Open node details for the recorded error.
            </p>
            <details>
              <summary>Run &amp; output details</summary>
              <p>Fixture · node / invocation / frame / attempt bound</p>
            </details>
          </div>
          <div v-else class="output">
            {{
              tab === 'Chat'
                ? 'Independent supervision conversation. Main Chat remains unchanged.'
                : tab === 'Requests'
                  ? 'Control receipts and user requests remain available here.'
                  : 'Recorded supervision reviews.'
            }}
          </div>
        </aside>
      </div>
    </template>
    <template v-else
      ><PageHeader title="Project Memory" />
      <div class="memory">
        <aside aria-label="Memory entries">
          <button
            v-for="i in 40"
            :key="i"
            :class="{ active: selected === i }"
            @click="selected = i"
          >
            <small>Project note · r1</small><strong>Normalization convention {{ i }}</strong
            ><span>Current · User note</span></button
          ><button @click="notice = 'Fixture next page loaded'">Load more</button>
        </aside>
        <article>
          <h2>Normalization convention {{ selected || 1 }}</h2>
          <p v-for="i in 14" :key="i">
            Preserve caller behavior and Chinese messages. Evidence and source versions remain
            inspectable.
          </p>
        </article>
      </div></template
    >
    <p v-if="notice" class="preview-notice" role="status">{{ notice }}</p>
    <div v-if="recovery" class="recovery-overlay">
      <section role="dialog" aria-label="Recovery copies">
        <header>
          <strong>Recovery copies</strong
          ><button aria-label="Close recovery copies" @click="recovery = false">
            <X :size="16" />
          </button>
        </header>
        <p>fix-normalize.blueprint · Backed up on host</p>
        <button
          class="btn"
          @click="
            text = saved
            notice = 'Fixture recovery copy selected'
            recovery = false
          "
        >
          Review this copy
        </button>
        <details>
          <summary>Draft options</summary>
          <button class="btn" @click="notice = 'Fixture backup retried'">Back up now</button
          ><button class="btn" @click="notice = 'Fixture draft retained pending explicit discard'">
            Discard draft…
          </button>
        </details>
      </section>
    </div>
  </div>
</template>
<style scoped>
.iu-preview {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--surface);
  font-size: 14px;
}
.preview-controls {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--border);
  font-size: 12px;
  flex-wrap: wrap;
}
.preview-controls > span {
  margin-right: auto;
  color: var(--muted-foreground);
}
button,
select {
  cursor: pointer;
}
select {
  background: var(--surface);
}
button:focus-visible,
input:focus-visible,
textarea:focus-visible {
  outline: 2px solid var(--ring);
}
.active {
  color: var(--primary) !important;
}
.editor-toolbar {
  display: flex;
  align-items: center;
  height: 44px;
  gap: 9px;
  padding: 0 12px;
  border-bottom: 1px solid var(--divider);
}
.editor-toolbar select {
  font-size: 13px;
  max-width: 210px;
}
.editor-toolbar > button,
.icon {
  display: inline-grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 5px;
  color: var(--muted-foreground);
}
.editor-toolbar > button:hover,
.icon:hover {
  background: var(--hover);
}
.spacer {
  flex: 1;
}
.muted,
small {
  color: var(--muted-foreground);
}
small {
  font-size: 11px;
}
.canvas {
  position: relative;
  flex: 1;
  min-height: 300px;
  overflow: auto;
  background-color: var(--background);
  background-image: radial-gradient(var(--grid-dot) 1px, transparent 1px);
  background-size: 20px 20px;
}
.preview-node {
  position: absolute;
  top: 95px;
  left: 45px;
  width: 228px;
  border-radius: 8px;
  background: var(--surface);
  box-shadow: var(--shadow-card);
  z-index: 1;
}
.preview-node > header {
  height: 26px;
  background: #8b5cf6;
  color: white;
  border-radius: 8px 8px 0 0;
  display: grid;
  place-items: center;
  font-size: 12px;
  font-weight: 600;
}
.preview-node label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin: 8px 12px;
  font-size: 12px;
}
.preview-node label > span {
  display: flex;
  justify-content: space-between;
}
.preview-node input:not([type='checkbox']) {
  height: 24px;
  border: 1px solid var(--border);
  border-radius: 4px;
  background: var(--input);
  padding: 0 5px;
  min-width: 0;
}
.preview-node input[type='checkbox'],
.inspector input[type='checkbox'] {
  width: 15px;
  height: 15px;
  margin: 0;
  align-self: flex-start;
  accent-color: var(--primary);
}
.pin-row {
  padding: 9px 12px 0;
}
.exec {
  display: block;
  width: 10px;
  height: 10px;
  border: 1.5px solid var(--exec-wire);
  clip-path: polygon(0 0, 100% 50%, 0 100%);
}
.pin-output {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 10px;
  font-size: 12px;
}
.pin-output i {
  width: 8px;
  height: 8px;
  border: 2px solid #a78bfa;
  border-radius: 50%;
  align-self: center;
}
.validator {
  left: 340px;
  top: 150px;
  width: 190px;
}
.validator header {
  background: #2fbf8f;
}
.preview-wire {
  position: absolute;
  top: 150px;
  left: 268px;
  width: 76px;
  height: 65px;
  border-top: 2px solid var(--exec-wire);
  border-right: 2px solid var(--exec-wire);
  border-radius: 20px;
}
.inspector {
  position: absolute;
  right: 12px;
  top: 12px;
  width: 280px;
  border: 1px solid var(--border);
  background: var(--surface-glass);
  border-radius: 9px;
  box-shadow: var(--shadow-popover);
  padding: 12px;
  z-index: 4;
}
.inspector header {
  display: flex;
  gap: 8px;
  align-items: center;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--divider);
}
.inspector label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin: 14px 0;
}
.inspector label > span {
  display: flex;
  justify-content: space-between;
}
.inspector p {
  font-size: 12px;
}
.chat-example {
  width: min(760px, 100%);
  margin: 0 auto;
  padding: 35px 30px;
  overflow: auto;
  flex: 1;
}
.user-text {
  background: var(--surface-muted);
  padding: 15px;
  border-radius: 10px;
  margin-bottom: 24px;
}
.step-summary {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--muted-foreground);
  font-size: 13px;
}
.rotated {
  transform: rotate(90deg);
}
.step-body {
  margin: 12px 0 14px 6px;
  padding-left: 12px;
  border-left: 1px solid var(--border);
  color: var(--muted-foreground);
  font-size: 13px;
}
.step-body p {
  display: flex;
  gap: 7px;
}
.step-body details {
  margin: 10px 0;
}
.answer {
  margin-top: 20px;
  line-height: 1.8;
}
.composer {
  width: min(760px, 100%);
  margin: 0 auto;
  padding: 18px 24px;
}
.composer textarea {
  width: 100%;
  resize: vertical;
  min-height: 80px;
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 12px;
  background: var(--input);
}
.composer > div {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  font-size: 12px;
}
.execution-body {
  display: flex;
  flex: 1;
  min-height: 0;
}
.execution-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.supervision {
  width: 400px;
  min-width: 270px;
  display: flex;
  flex-direction: column;
  border-left: 1px solid var(--border);
  background: var(--background);
}
.supervision > header {
  display: flex;
  gap: 8px;
  padding: 15px 18px;
  font-size: 15px;
}
.supervision nav {
  display: flex;
  gap: 20px;
  padding: 0 18px;
  height: 42px;
  align-items: center;
  border-bottom: 1px solid var(--divider);
  font-size: 13px;
}
.output {
  overflow: auto;
  flex: 1;
  padding: 18px;
  line-height: 1.7;
}
.output-label {
  font-size: 13px;
  font-weight: 600;
  display: flex;
  gap: 12px;
  margin: 5px 0 14px;
}
.output-label span {
  margin-left: auto;
  font-size: 11px;
  font-weight: 400;
  color: var(--muted-foreground);
}
.output pre {
  padding: 12px;
  background: var(--surface-muted);
  border-radius: 6px;
  white-space: pre-wrap;
  font-size: 12px;
}
.output details {
  font-size: 12px;
  color: var(--muted-foreground);
}
.runtime {
  top: 60px;
  left: 50px;
}
.literal {
  font-size: 12px;
  line-height: 1.5;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  margin: 0;
}
.bottom-detail {
  padding: 16px 20px;
  border-top: 1px solid var(--border);
  font-size: 12px;
  max-height: 220px;
  overflow: auto;
}
.bottom-detail p {
  margin: 7px 0;
}
.run-state {
  font-size: 13px;
  color: var(--muted-foreground);
}
.failure {
  color: var(--danger);
}
.memory {
  display: flex;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  margin: 0 24px 24px;
  border: 1px solid var(--border);
  border-radius: 8px;
}
.memory > aside {
  width: 260px;
  flex-shrink: 0;
  overflow: auto;
  border-right: 1px solid var(--border);
  background: var(--sidebar);
  padding: 12px;
}
.memory > aside button {
  display: flex;
  flex-direction: column;
  width: 100%;
  gap: 6px;
  padding: 12px;
  text-align: left;
  border-radius: 7px;
}
.memory > aside button.active {
  background: var(--surface);
}
.memory strong {
  font-size: 13px;
}
.memory span {
  font-size: 12px;
  color: var(--muted-foreground);
}
.memory article {
  overflow: auto;
  flex: 1;
  padding: 24px;
  line-height: 1.8;
}
.memory h2 {
  font-size: 18px;
}
.preview-notice {
  padding: 8px 16px;
  background: var(--primary-soft);
  font-size: 13px;
}
.recovery-overlay {
  position: fixed;
  inset: 0;
  background: #0005;
  display: grid;
  place-items: center;
  z-index: 100;
}
.recovery-overlay > section {
  width: min(500px, 90vw);
  padding: 22px;
  background: var(--popover);
  border: 1px solid var(--border);
  border-radius: 10px;
}
.recovery-overlay header {
  display: flex;
  justify-content: space-between;
}
.recovery-overlay p,
.recovery-overlay details {
  margin: 15px 0;
}
@media (max-width: 1050px) {
  .validator {
    left: 300px;
  }
  .inspector {
    width: 240px;
  }
  .supervision {
    width: 340px;
  }
  .preview-node {
    left: 20px;
    width: 205px;
  }
}
@media (max-width: 760px) {
  .preview-controls {
    gap: 7px;
  }
  .editor-toolbar {
    gap: 4px;
  }
  .editor-toolbar .muted {
    display: none;
  }
  .editor-toolbar select {
    max-width: 130px;
  }
  .validator {
    left: 280px;
  }
  .inspector {
    position: relative;
    right: auto;
    top: auto;
    margin: 350px 12px 12px;
    width: auto;
  }
  .execution-body {
    flex-direction: column;
    overflow: auto;
  }
  .execution-main {
    min-height: 500px;
  }
  .supervision {
    width: 100%;
    min-height: 450px;
    border-left: 0;
    border-top: 1px solid var(--border);
  }
  .memory {
    margin: 0 12px 12px;
  }
  .memory > aside {
    width: 180px;
  }
  .memory article {
    padding: 14px;
  }
}
</style>
