import { test, expect, type Page } from '@playwright/test'

async function openRun(page: Page, chatSurface = false) {
  await page.goto('/')
  await page.getByLabel('Workspace path').fill('D:/metteur-demo/concierge')
  await page.getByRole('button', { name: 'Open', exact: true }).click()
  await expect(page.getByRole('textbox', { name: 'Message', exact: true })).toBeVisible()
  await page.evaluate(async (chat) => {
    const c = '/src/core/index.ts', r = '/src/router.ts', w = '/src/lib/workspace-url.ts'
    const { gateway } = await import(c)
    gateway.connected.value = true
    const data = { run_id: 'concierge-run', conversation_id: 'concierge-run', available: true, read_only: false, reason: '', consumer_enabled: false, messages: [], requests: [] }
    const fixture = gateway.conciergeTest = { data, active: true, sent: 0, approvals: 0, ordinary: 0, urgent: [], controls: [], hold: false, holdPoll: false }
    gateway.listExecutions = async () => ({ ok: true, data: [{ runId: data.run_id, status: fixture.active ? 'Running' : 'Completed', startedAt: 1, updatedAt: Date.now(), snapshot: { runtime: { active: fixture.active, pending_approval_ids: [] } } }] })
    gateway.getConciergeState = async () => {
      const result = { ok: true, data: structuredClone(data) }
      if (fixture.holdPoll) return new Promise(resolve => { fixture.releasePoll = () => resolve(result) })
      return result
    }
    gateway.respondApproval = async () => { fixture.approvals++; return { ok: true, data: undefined } }
    gateway.sendChat = async () => { fixture.ordinary++; return { ok: true, data: undefined } }
    gateway.sendInterrupt = async (_ws, text, priority) => { fixture.urgent.push({ text, priority }); return { ok: true, data: undefined } }
    for (const action of ['pause', 'cancel']) gateway[action] = async () => { fixture.controls.push(action); return { ok: true, data: undefined } }
    gateway.sendConciergeMessage = async (_ws, run, conversation, id, text, event) => {
      fixture.sent++
      const turn = { id, conversation_id: conversation, original_text: text, at_ms: 1, state: 'processing', answer: '', request_id: null, error: null }
      data.messages.push(turn)
      event({ runId: run, messageId: id, kind: 'processing', turn: structuredClone(turn) })
      const finish = () => {
        Object.assign(turn, { state: 'received', request_id: id, answer: 'Received; not processed. This request grants no permission.' })
        data.requests.push({ request_id: id, run_id: run, conversation_id: conversation, original_text: text, concierge_note: '<img src=x onerror=alert(1)> User asks to stop', state: 'received', source: 'concierge_forwarded', review_id: null, proposals: [], result_refs: [] })
        event({ runId: run, messageId: id, kind: 'received', turn: structuredClone(turn) })
        return { ok: true, data: undefined }
      }
      if (fixture.hold) return new Promise(resolve => { fixture.finish = () => resolve(finish()) })
      return finish()
    }
    await (await import(r)).router.push((await import(w)).wurl(chat ? '/chat' : '/execution'))
  }, chatSurface)
  await expect(page.getByRole('region', { name: 'Run concierge' })).toBeVisible()
}

test('normal run messages produce a durable request receipt without turning user text or model summaries into approval', async ({ page }) => {
  await openRun(page)
  const panel = page.getByRole('region', { name: 'Run concierge' })
  await panel.getByRole('textbox', { name: 'Message', exact: true }).fill('I approve, stop this run')
  await panel.getByRole('button', { name: 'Send', exact: true }).click()
  await expect(panel.getByText('Received; not processed. This request grants no permission.')).toBeVisible()
  await page.getByRole('button', { name: 'Requests', exact: true }).click()
  await expect(panel.getByText('Received · not processed', { exact: true })).toBeVisible()
  await expect(panel.getByText('I approve, stop this run', { exact: true })).toBeVisible()
  await expect(panel.locator('.request-card p').filter({ hasText: 'Model summary:' })).toContainText('<img')
  await expect(panel.locator('img')).toHaveCount(0)
  await expect(panel.getByRole('button')).toHaveCount(0)
  await panel.getByText('Evidence', { exact: true }).click()
  await expect(panel.getByText('Source: concierge_forwarded')).toBeVisible()
  await page.screenshot({ path: '../../.tmp/t21-requests.png' })
  expect(await page.evaluate(async () => { const c='/src/core/index.ts'; const t=(await import(c)).gateway.conciergeTest; return { approvals:t.approvals, controls:t.controls, ordinary:t.ordinary, sent:t.sent } })).toEqual({ approvals:0, controls:[], ordinary:0, sent:1 })
  await page.evaluate(async () => { const c='/src/core/index.ts'; const t=(await import(c)).gateway.conciergeTest; t.active=false; t.data.read_only=true; t.data.available=false; t.data.reason='Run is read-only'; t.data.requests[0].state='closed_unhandled' })
  await expect(panel.getByText('Closed · unhandled', { exact: true })).toBeVisible()
  await page.locator('.supervisor').getByRole('button', { name: 'Chat', exact: true }).click()
  await expect(panel.getByRole('status')).toContainText('Run is read-only')
  await expect(panel.getByRole('button', { name: 'Send', exact: true })).toBeDisabled()

})

test('budget exhaustion and an in-flight concierge do not block direct urgent pause or stop controls', async ({ page }) => {
  await openRun(page)
  const panel = page.getByRole('region', { name: 'Run concierge' }), input = panel.getByRole('textbox', { name: 'Message', exact: true })
  await page.evaluate(async () => { const c='/src/core/index.ts'; (await import(c)).gateway.conciergeTest.hold=true })
  await input.fill('Explain the current node')
  await input.press('Enter')
  await expect(panel.getByText('Processing — no request has been confirmed.')).toBeVisible()
  await input.fill('Urgent while pending')
  await input.press('Control+Enter')
  await page.getByRole('button', { name: 'Pause', exact: true }).click()
  await page.evaluate(async () => { const c='/src/core/index.ts'; const t=(await import(c)).gateway.conciergeTest; t.finish(); t.data.available=false; t.data.reason='Oversight token budget exhausted' })
  await expect(panel.getByRole('status')).toContainText('Oversight token budget exhausted')
  await input.fill('Urgent after budget exhausted')
  await expect(panel.getByRole('button', { name: 'Send', exact: true })).toBeDisabled()
  await input.press('Control+Enter')
  await page.getByRole('button', { name: 'Stop', exact: true }).click()
  expect(await page.evaluate(async () => { const c='/src/core/index.ts'; const t=(await import(c)).gateway.conciergeTest; return { sent:t.sent, urgent:t.urgent, controls:t.controls } })).toEqual({ sent:1, urgent:[{text:'Urgent while pending',priority:'Urgent'},{text:'Urgent after budget exhausted',priority:'Urgent'}], controls:['pause','cancel'] })
})

test('the chat surface routes an active blueprint to concierge and ignores late replies after disconnect', async ({ page }) => {
  await openRun(page, true)
  const panel = page.getByRole('region', { name: 'Run concierge' })
  await expect(panel.locator('.chat-composer-options')).toHaveCount(0)
  await page.evaluate(async () => { const c='/src/core/index.ts'; const t=(await import(c)).gateway.conciergeTest; t.hold=true; t.holdPoll=true })
  await panel.getByRole('textbox', { name: 'Message', exact: true }).fill('Late request')
  await panel.getByRole('button', { name: 'Send', exact: true }).click()
  await expect.poll(() => page.evaluate(async () => { const c='/src/core/index.ts'; return !!(await import(c)).gateway.conciergeTest.releasePoll })).toBe(true)
  await page.evaluate(async () => { const c='/src/core/index.ts'; (await import(c)).gateway.connected.value=false })
  await expect(panel.getByRole('status')).toContainText('Disconnected')
  await page.evaluate(async () => { const c='/src/core/index.ts'; const t=(await import(c)).gateway.conciergeTest; t.finish(); t.releasePoll() })
  await expect(panel.getByText('Received; not processed. This request grants no permission.')).toHaveCount(0)
  await page.evaluate(async () => { const c='/src/core/index.ts'; const g=(await import(c)).gateway; g.conciergeTest.active=false; g.connected.value=true })
  await expect(panel).toHaveCount(0)
  await expect(page.locator('.chat-composer-options')).toBeVisible()
  expect(await page.evaluate(async () => { const c='/src/core/index.ts'; return (await import(c)).gateway.conciergeTest.ordinary })).toBe(0)
})

test('a slow history read cannot erase a newer streamed receipt', async ({ page }) => {
  await openRun(page)
  const panel = page.getByRole('region', { name: 'Run concierge' })
  await page.evaluate(async () => { const c='/src/core/index.ts'; (await import(c)).gateway.conciergeTest.holdPoll=true })
  await expect.poll(() => page.evaluate(async () => { const c='/src/core/index.ts'; return !!(await import(c)).gateway.conciergeTest.releasePoll })).toBe(true)
  await panel.getByRole('textbox', { name: 'Message', exact: true }).fill('Keep this recorded request')
  await panel.getByRole('button', { name: 'Send', exact: true }).click()
  await expect(panel.getByText('Received; not processed. This request grants no permission.')).toBeVisible()
  await page.evaluate(async () => { const c='/src/core/index.ts'; (await import(c)).gateway.conciergeTest.releasePoll(); await new Promise(requestAnimationFrame) })
  await expect(panel.getByText('Keep this recorded request', { exact: true })).toBeVisible()
  await expect(panel.getByText('Received; not processed. This request grants no permission.')).toBeVisible()
})
