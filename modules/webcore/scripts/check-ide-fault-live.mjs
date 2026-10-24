import { createClient } from '@connectrpc/connect'
import { createGrpcWebTransport } from '@connectrpc/connect-web'
import { Daemon } from '../src/gen/metteur_pb.ts'
import { chromium, expect } from '@playwright/test'
import { readFile, writeFile, mkdir } from 'node:fs/promises'
const [readyPath, output, priorPath] = process.argv.slice(2),
  ready = JSON.parse(await readFile(readyPath, 'utf8')),
  prior = priorPath ? JSON.parse(await readFile(priorPath, 'utf8')) : null,
  workspacePath = prior?.workspace ?? ready.root + '/fault-' + Date.now() + '/workspace',
  rpc = createClient(Daemon, createGrpcWebTransport({ baseUrl: ready.baseURL })),
  report = { checks: [], events: [], errors: [], workspace: workspacePath, reused: priorPath }
if (!prior) await mkdir(workspacePath, { recursive: true })
await rpc.openWorkspace({ path: workspacePath })
let run
if (!prior) {
  const graph = await rpc.compileDsl({
    workspacePath,
    source:
      'blueprint "IU actual failure"\nentry start: Start\nllm: CallLLM(model = "fixture", prompt = "IU_FORCE_FAILURE")\nend: End\nstart -> llm\nllm -> end\n',
  })
  await rpc.saveBlueprint({ workspacePath, blueprint: graph, filePath: 'fault.blueprint' })
  try {
    for await (const e of rpc.executeBlueprint(
      { workspacePath, blueprintId: graph.id },
      { timeoutMs: 20000 },
    ))
      report.events.push({
        kind: e.kind,
        message: e.message,
        detail: JSON.parse(e.detailJson || '{}'),
      })
  } catch (e) {
    report.transport_error = String(e)
  }
  run = (await rpc.listExecutions({ workspacePath })).executions.find(
    (r) => r.blueprintId === graph.id,
  )
} else
  run = (await rpc.listExecutions({ workspacePath })).executions.find((r) => r.runId === prior.run)
expect(run.status).toBe('Failed')
report.run = run.runId
const browser = await chromium.launch(),
  page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, serviceWorkers: 'block' })
page.on('pageerror', (e) => report.errors.push(String(e)))
try {
  await page.goto(ready.baseURL + '/?open=' + encodeURIComponent(workspacePath))
  await page.waitForURL(/\/work\/[^/]+\//)
  await page
    .getByRole('navigation', { name: 'Main navigation' })
    .getByRole('button', { name: 'Execution', exact: true })
    .click()
  await expect(page.getByText(/Run needs attention/)).toBeVisible()
  await expect(page.getByRole('region', { name: 'Node output' })).toContainText(/503|[Ff]ailed/)
  await page.getByRole('alert').filter({hasText:'Run needs attention'}).click()
  await page.getByText('Recorded run diagnostics',{exact:true}).click()
  await expect(page.locator('details[open] pre').filter({hasText:'503'})).toBeVisible()
  await expect(page.getByRole('region', { name: 'Node output' })).toContainText('503')
  await page.screenshot({ path: output + '.png' })
  report.checks.push(
    'A real local-provider 503 failure stays explicit in the compact header and Output; raw diagnostic remains accessible in Execution log; no paid provider call',
  )
  expect(report.errors).toHaveLength(0)
} catch (e) {
  report.failure = String(e)
  await writeFile(output + '.page.txt', await page.locator('body').innerText())
  await page.screenshot({ path: output + '.failure.png' })
}
await writeFile(output + '.json', JSON.stringify(report, null, 2))
await browser.close()
process.exit(report.failure || report.errors.length ? 1 : 0)
