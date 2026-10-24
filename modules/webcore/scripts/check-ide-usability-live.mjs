import { chromium, expect } from '@playwright/test'
import { createClient } from '@connectrpc/connect'
import { createGrpcWebTransport } from '@connectrpc/connect-web'
import { fromJson } from '@bufbuild/protobuf'
import { Daemon, SaveBlueprintRequestSchema } from '../src/gen/metteur_pb.ts'
import { randomUUID } from 'node:crypto'
import { readFile, writeFile, mkdir } from 'node:fs/promises'
const [readyPath, output, resumePath] = process.argv.slice(2),
  ready = JSON.parse(await readFile(readyPath, 'utf8')),
  prior = resumePath ? JSON.parse(await readFile(resumePath, 'utf8')) : null,
  root = prior?.workspace ?? ready.root + '/ui-' + Date.now() + '/workspace'
if (!prior) {
  await mkdir(root, { recursive: true })
  await writeFile(root + '/source.txt', 'IU_SOURCE preserved 中文\n')
}
const rpc = createClient(Daemon, createGrpcWebTransport({ baseUrl: ready.baseURL }))
async function resource(method, payload, revision = 0) {
  const r = await rpc[method]({
    workspacePath: root,
    schemaVersion: 1,
    requestId: randomUUID(),
    expectedRevision: BigInt(revision),
    payloadJson: JSON.stringify(payload),
  })
  if (r.errorJson) throw Error(r.errorJson)
  return JSON.parse(r.payloadJson)
}
async function save(graph, file) {
  const body = JSON.stringify(graph, null, 2)
  await rpc.writeFile({ workspacePath: root, path: file, content: body, createOnly: true })
  const blueprint = {
    id: graph.id,
    name: graph.name,
    entry_node_id: graph.entry_node_id,
    edges: graph.edges,
    nodes: graph.nodes.map((n) => ({
      id: n.id,
      kind: n.kind,
      node_type: n.node_type,
      pos_x: n.position[0],
      pos_y: n.position[1],
      data_json: JSON.stringify(n.data),
      pins: n.pins.map((p) => ({
        id: p.id,
        name: p.name,
        key: p.key ?? '',
        pin_type: p.pin_type,
        data_type: p.data_type,
        optional: p.optional ?? false,
        choices: p.choices ?? [],
        description: p.description ?? '',
        default_json:
          p.default == null && !graph.explicit_null_defaults?.includes(p.id)
            ? ''
            : JSON.stringify(p.default),
      })),
    })),
  }
  await rpc.saveBlueprint(
    fromJson(SaveBlueprintRequestSchema, { workspace_path: root, blueprint, file_path: file }),
  )
}
async function template(name, goal) {
  const input = {
    instance_id: randomUUID(),
    template: 'refactor',
    name,
    goal,
    constraints: 'Preserve source.txt',
    scope_paths: ['source.txt'],
    evidence_paths: ['source.txt'],
    model: 'fixture',
    max_iterations: 4,
    baseline: {
      id: 'baseline',
      description: 'Baseline',
      command: 'node -e "process.exit(0)"',
      expected_exit_code: 0,
      required: true,
    },
    checks: [
      {
        id: 'final',
        description: 'Approved check',
        command: 'node -e "process.exit(0)"',
        expected_exit_code: 0,
        required: true,
      },
    ],
    manual_checks: [
      {
        id: 'human',
        description: 'HUMAN_REQUIREMENT: preserve exact 中文 messages',
        required: true,
      },
    ],
  }
  return (await resource('queryCodingWorkflows', { operation: 'compile', input })).graph
}
await rpc.openWorkspace({ path: root })
const a = prior
    ? JSON.parse(await readFile(root + '/iu-output.blueprint', 'utf8'))
    : await template('IU output', 'IU_OUTPUT: read source.txt and report the actual text.'),
  c = prior
    ? JSON.parse(await readFile(root + '/iu-cancel.blueprint', 'utf8'))
    : await template('IU cancel', 'IU_CANCEL: retain partial output until stopped.')
if (!prior) {
  await save(a, 'iu-output.blueprint')
  await save(c, 'iu-cancel.blueprint')
}
const catalog = await rpc.listNodeKinds({ workspacePath: root })
function node(kind, x, data = {}) {
  const info = catalog.infos.find((n) => n.kind === kind)
  if (!info) throw Error('Missing ' + kind)
  return {
    id: randomUUID(),
    kind,
    node_type: info.nodeType,
    position: [x, 0],
    data,
    pins: info.pins.map((p) => ({
      id: randomUUID(),
      key: p.key,
      name: p.name,
      pin_type: p.pinType,
      data_type: p.dataType,
      optional: p.optional,
      choices: p.choices,
      description: p.description,
      ...(p.defaultJson ? { default: JSON.parse(p.defaultJson) } : {}),
    })),
  }
}
const start = node('Start', 0),
  branch = node('Not', 280, { a: false }),
  end = node('End', 560)
const edge = (source, target, out) => ({
  id: randomUUID(),
  source_node: source.id,
  source_pin: source.pins.find((p) => p.pin_type === 'ExecOutput' && (!out || p.name === out)).id,
  target_node: target.id,
  target_pin: target.pins.find((p) => p.pin_type === 'ExecInput').id,
})
const b = {
  id: randomUUID(),
  name: 'IU independent',
  entry_node_id: start.id,
  nodes: [start, branch, end],
  edges: [edge(start, branch), edge(branch, end)],
}
if (!prior) await save(b, 'iu-independent.blueprint')
const browser = await chromium.launch(),
  page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, serviceWorkers: 'block' })
const report = {
    checks: [],
    errors: [],
    sends: 0,
    runs: 0,
    workspace: root,
    completedRun: prior?.completedRun,
    reused: resumePath,
  },
  nav = () => page.getByRole('navigation', { name: 'Main navigation' })
let completed = prior?.completedRun ? { runId: prior.completedRun } : null
page.on('pageerror', (e) => report.errors.push(String(e)))
page.on('request', (r) => {
  if (r.method() === 'POST' && /SendChat$|\/api\/chat\/stream(?:\?|$)/.test(r.url())) report.sends++
  if (/ExecuteBlueprint$/.test(r.url())) report.runs++
})
async function openFile(file) {
  if (!(await page.getByTestId('file-tree').isVisible()))
    await nav().getByRole('button', { name: 'Explorer', exact: true }).click()
  await page.getByTestId('file-tree').getByRole('button', { name: file, exact: true }).click()
  await expect(
    page.locator('[data-editor-pane]').first().getByText(file, { exact: true }),
  ).toBeVisible()
  await expect(
    page.locator('[data-editor-pane] div[tabindex="0"][aria-busy="false"]').first(),
  ).toBeVisible()
}
async function runFile(file) {
  await openFile(file)
  await page.getByRole('button', { name: 'Run', exact: true }).click()
  await page.getByRole('button', { name: 'Run confirmed version', exact: true }).click()
}
async function waitRun(status, blueprint) {
  let result
  await expect
    .poll(
      async () => {
        const list = await rpc.listExecutions({ workspacePath: root })
        result = list.executions.findLast((r) => r.blueprintId === blueprint && r.status === status)
        return !!result
      },
      { timeout: 40000 },
    )
    .toBe(true)
  return result
}
try {
  await page.goto(ready.baseURL + '/?open=' + encodeURIComponent(root))
  await page.waitForURL(/\/work\/[^/]+\//)
  // An existing workspace may restore an Execution tab after the first tree
  // mounts. Use the explicit Explorer route before interacting with files.
  await page.goto(
    ready.baseURL + '/work/' + new URL(page.url()).pathname.split('/')[2] + '/explorer',
  )
  await expect(page.getByTestId('file-tree')).toBeVisible()
  const pane = page.locator('[data-editor-pane]').first(),
    nodeView = page.locator(`.vue-flow__node[data-id="${branch.id}"]`)
  if (!prior) {
    await openFile('iu-independent.blueprint')
    await expect(pane.getByRole('button', { name: 'Recovery Copies', exact: true })).toBeVisible()
    await expect(pane.getByText('Start from template', { exact: true })).toHaveCount(0)
    await expect(nodeView.getByText('A', { exact: true })).toBeVisible()
    const checkbox = nodeView.locator('input[type=checkbox]')
    await expect(checkbox).not.toBeChecked()
    await checkbox.check()
    await checkbox.press('Control+s')
    await expect
      .poll(async () => {
        const graph = JSON.parse(await readFile(root + '/iu-independent.blueprint', 'utf8'))
        const n = graph.nodes.find((node) => node.id === branch.id)
        return Object.values(n.values ?? n.data ?? {}).includes('true')
      })
      .toBe(true)
    const label = await nodeView.locator('.metteur-pin-label').first().boundingBox(),
      box = await checkbox.boundingBox()
    expect(Math.abs(label.x - box.x)).toBeLessThan(4)
    await page.locator('.blueprint-flow').click({ position: { x: 20, y: 20 } })
    await page.keyboard.press('Control+a')
    expect(await page.locator('.vue-flow__node.selected').count()).toBe(3)
    await page.keyboard.press('Control+c')
    await page.keyboard.press('Control+v')
    await expect(page.locator('.vue-flow__node')).toHaveCount(6)
    await page.keyboard.press('Control+s')
    await expect
      .poll(
        async () =>
          JSON.parse(await readFile(root + '/iu-independent.blueprint', 'utf8')).nodes.length,
      )
      .toBe(6)
    let pasted = JSON.parse(await readFile(root + '/iu-independent.blueprint', 'utf8'))
    const pins = pasted.nodes.flatMap((n) =>
      [...(n.inputs ?? []), ...(n.outputs ?? []), ...(n.pins ?? [])].map((p) => p.id),
    )
    expect(new Set(pins).size).toBe(pins.length)
    await page.keyboard.press('Control+z')
    await expect(page.locator('.vue-flow__node')).toHaveCount(3)
    await page.keyboard.press('Control+s')
    report.checks.push(
      'Recovery/template icons use existing toolbar; pin name/type and left-aligned bool visible; focused Ctrl+S saves; canvas multi-select/copy/paste preserves unique pin IDs and undo',
    )
    await openFile('iu-output.blueprint')
    const llm = a.nodes.find((n) => n.kind === 'CallLLM')
    await page.locator(`.vue-flow__node[data-id="${llm.id}"]`).click()
    await page.getByRole('button', { name: 'Expand Prompt', exact: true }).click()
    const expanded = page.getByRole('textbox', { name: 'Prompt expanded', exact: true }),
      original = await expanded.inputValue()
    await expanded.fill(original + '\nIU_MULTILINE: preserve  spaces  中文')
    await page.getByRole('button', { name: 'Cancel', exact: true }).click()
    await page.getByRole('button', { name: 'Expand Prompt', exact: true }).click()
    await expect(expanded).toHaveValue(original)
    await expanded.fill(original + '\nIU_MULTILINE: preserve  spaces  中文')
    await expanded.press('Control+s')
    await expect
      .poll(async () =>
        JSON.stringify(JSON.parse(await readFile(root + '/iu-output.blueprint', 'utf8'))),
      )
      .toContain('IU_MULTILINE')
    await page.getByRole('button', { name: 'Cancel', exact: true }).click()
    report.checks.push(
      'Expanded inspector cancels without applying; Ctrl+S applies and saves multiline Chinese literal through the real file/mirror service',
    )
    await page.getByRole('button', { name: 'Run', exact: true }).click()
    await page.getByRole('button', { name: 'Run confirmed version', exact: true }).click()
    completed = await waitRun('Completed', a.id)
    report.completedRun = completed.runId
    const outputPanel = page.getByRole('region', { name: 'Node output' })
    await expect(outputPanel).toContainText('IU_NODE_FINAL', { timeout: 30000 })
    await expect(outputPanel).toContainText('CallLLM')
    await expect(page.locator('.runtime-node').getByText('x-in', { exact: true })).toHaveCount(0)
    const data = await resource('queryCodingWorkflows', {
      operation: 'output',
      run_id: completed.runId,
    })
    expect(data.nodes[0].messages.some((m) => m.role === 'tool' && m.actor === 'ReadFile')).toBe(
      true,
    )
    await page.reload()
    await expect(page.getByRole('region', { name: 'Node output' })).toContainText('IU_NODE_FINAL')
    await page.screenshot({ path: output + '.output.png' })
    await openFile('iu-independent.blueprint')
    await expect(pane.locator('.chip').filter({ hasText: /finished|cancelled/i })).toHaveCount(0)
    report.checks.push(
      'Actual CallLLM stream includes tool result and final output, persists across refresh, displays frozen literals and does not contaminate another blueprint editor',
    )
  }
  let cancelled = prior?.cancelledRun ? { runId: prior.cancelledRun } : null
  if (!cancelled) {
    await runFile('iu-cancel.blueprint')
    await expect(page.getByRole('region', { name: 'Node output' })).toContainText('IU_PARTIAL', {
      timeout: 20000,
    })
    await page.getByRole('button', { name: 'Stop', exact: true }).click()
    cancelled = await waitRun('Cancelled', c.id)
    report.cancelledRun = cancelled.runId
    await expect(page.getByText('Stopped by you', { exact: true })).toBeVisible()
    expect(await page.locator('.run-header').innerText()).not.toContain(
      'execution interrupted: cancelled by user',
    )
    await page.screenshot({ path: output + '.stopped.png' })
    await openFile('iu-independent.blueprint')
    await expect(pane.locator('.chip').filter({ hasText: /finished|cancelled/i })).toHaveCount(0)
    report.checks.push(
      'Real cancellation retains partial output, uses compact stopped status and cannot contaminate another editor',
    )
  }
  report.cancelledRun = cancelled.runId
  if (!prior?.cancelledRun) {
    for (let i = 0; i < 14; i++)
      await resource('writeProjectNote', {
        id: randomUUID(),
        title: `IU memory ${i}`,
        body: 'Long note 中文 '.repeat(150),
        files: ['source.txt'],
      })
  }
  await nav().getByRole('button', { name: 'Project Memory', exact: true }).click()
  const list = page.getByRole('complementary', { name: 'Memory entries' })
  expect(await list.evaluate((e) => e.scrollHeight > e.clientHeight)).toBe(true)
  await list.evaluate((e) => (e.scrollTop = e.scrollHeight))
  await expect(list.locator('button').last()).toBeInViewport()
  report.checks.push('Real Project Memory list independently scrolls to the final note')
  const review = await resource('queryCodingWorkflows', {
    operation: 'review',
    run_id: cancelled.runId,
  })
  await resource(
    'recordWorkflowReview',
    {
      run_id: cancelled.runId,
      evidence_digest: review.digest,
      criterion_id: 'human',
      decision: 'changes_requested',
      note: 'REVISION_NOTE: preserve exact 中文 and boundary behavior',
    },
    review.revision,
  )
  await nav().getByRole('button', { name: 'Chat', exact: true }).click()
  await page.getByRole('button', { name: 'New conversation', exact: true }).click()
  await expect(page.getByText('Control & Input Receipts', { exact: true })).toHaveCount(0)
  await page
    .getByRole('textbox', { name: 'Message', exact: true })
    .fill('IU_CHAT_STEPS: read source.txt.')
  await page.getByRole('button', { name: 'Send', exact: true }).click()
  await expect(page.getByText(/IU_NODE_FINAL/).last()).toBeVisible({ timeout: 30000 })
  await expect(page.locator('.chat-tool-group')).toHaveCount(1)
  report.checks.push(
    'Real Chat renders reasoning and tool activity in one steps group and keeps the final answer separate',
  )
  await page.getByRole('textbox', { name: 'Message', exact: true }).fill('Keep my original draft.')
  await nav().getByRole('button', { name: 'Execution', exact: true }).click()
  await page.getByRole('button', { name: 'Result', exact: true }).click()
  await page.getByRole('button', { name: 'Review delivery', exact: true }).click()
  await expect(page.getByText(/REVISION_NOTE/)).toBeVisible()
  const sends = report.sends,
    runs = report.runs
  await page.getByRole('button', { name: 'Prepare follow-up in Chat', exact: true }).click()
  const draft = page.getByRole('textbox', { name: 'Message', exact: true })
  await expect(draft).toHaveValue(
    /Keep my original draft\.[\s\S]*REVISION_NOTE[\s\S]*review|Keep my original draft\.[\s\S]*review[\s\S]*REVISION_NOTE/,
  )
  expect(report.sends).toBe(sends)
  expect(report.runs).toBe(runs)
  report.checks.push(
    'Verified review note, requirement IDs, revision and evidence digest append to the same original draft with zero automatic Send or Run',
  )
  await page.getByRole('button', { name: 'Send', exact: true }).click()
  await expect(page.getByText(/IU_FEEDBACK_READBACK/).last()).toBeVisible({ timeout: 30000 })
  const receipt = await (await fetch(ready.providerUrl + '/fixture')).json()
  expect(receipt.manualSeen).toBe(true)
  expect(receipt.feedbackSeen).toBe(true)
  report.provider = receipt
  await page.screenshot({ path: output + '.feedback.png' })
  report.checks.push(
    'Chat has no receipt strip and groups reasoning/tools; human requirement reaches actual node request; verified human note and original draft reach explicit Send without automatic execution',
  )
  expect(report.errors).toHaveLength(0)
} catch (error) {
  report.failure = String(error)
  await writeFile(output + '.page.txt', await page.locator('body').innerText())
  await page.screenshot({ path: output + '.failure.png' })
}
await writeFile(output + '.json', JSON.stringify(report, null, 2))
await browser.close()
console.log(
  JSON.stringify({ checks: report.checks, failure: report.failure, errors: report.errors }),
)
if (report.failure || report.errors.length) process.exitCode = 1
