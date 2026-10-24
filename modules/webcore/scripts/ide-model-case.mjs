// Actual model -> current discovery tools -> approved save -> independent run.
// A model's own answer is never the acceptance verdict. No automatic retry.
import { createClient } from '@connectrpc/connect'
import { createGrpcWebTransport } from '@connectrpc/connect-web'
import { Daemon } from '../src/gen/metteur_pb.ts'
import { readFile, writeFile } from 'node:fs/promises'
import { randomUUID, createHash } from 'node:crypto'
import { join } from 'node:path'
import assert from 'node:assert/strict'
const [readyPath, destination] = process.argv.slice(2)
const ready = JSON.parse(await readFile(readyPath, 'utf8')),
  spec = ready.case,
  workspacePath = ready.workspace,
  client = createClient(Daemon, createGrpcWebTransport({ baseUrl: ready.baseURL }))
const report = {
  id: spec.id,
  status: 'running',
  checks: [],
  events: [],
  approvals: [],
  started: new Date().toISOString(),
}
async function resource(method, payload) {
  const r = await client[method]({
    workspacePath,
    schemaVersion: 1,
    requestId: randomUUID(),
    payloadJson: JSON.stringify(payload),
  })
  if (r.errorJson) throw Error(r.errorJson)
  return JSON.parse(r.payloadJson)
}
function safeGraph(g) {
  assert(g.nodes.length <= 12)
  const allowed = new Set(spec.kinds)
  for (const n of g.nodes) {
    const kind = n.kind === 'Tool' ? n.data.tool_name : n.kind
    assert(allowed.has(kind), `Unexpected authored kind ${kind}`)
    if (kind === 'ReadFile')
      assert(
        n.data.path === 'evidence.txt' ||
          g.edges.some(
            (e) =>
              e.target_node === n.id &&
              n.pins.some((p) => p.id === e.target_pin && p.name === 'path'),
          ),
        'ReadFile must use synthetic source',
      )
    if (n.kind === 'Start')
      assert(
        Object.values(n.data).every((v) => v === 'evidence.txt'),
        'Synthetic Start constants only',
      )
    if (n.kind === 'CallLLM') {
      assert(n.data.model === 'flash')
      assert(n.data.max_iterations <= 4 && n.data.max_tokens <= 512)
      assert(/evidence.txt/.test(n.data.prompt))
    }
  }
}
async function proto(g) {
  return {
    id: g.id,
    name: g.name,
    entryNodeId: g.entry_node_id,
    nodes: g.nodes.map((n) => ({
      id: n.id,
      kind: n.kind,
      nodeType: n.node_type,
      posX: n.position[0],
      posY: n.position[1],
      dataJson: JSON.stringify(n.data),
      pins: n.pins.map((p) => ({
        id: p.id,
        key: p.key ?? '',
        name: p.name,
        pinType: p.pin_type,
        dataType: p.data_type,
        optional: p.optional ?? false,
        defaultJson:
          p.default == null && !g.explicit_null_defaults?.includes(p.id)
            ? ''
            : JSON.stringify(p.default),
        choices: p.choices ?? [],
        description: p.description ?? '',
      })),
    })),
    edges: g.edges.map((e) => ({
      id: e.id,
      sourceNode: e.source_node,
      sourcePin: e.source_pin,
      targetNode: e.target_node,
      targetPin: e.target_pin,
    })),
  }
}
try {
  await client.openWorkspace({ path: workspacePath })
  let graph, file
  if (spec.id === 'manual_requirement') {
    const check = (id) => ({
      id,
      description: 'Synthetic deterministic check',
      command: 'node -e "process.exit(0)"',
      expected_exit_code: 0,
      required: true,
    })
    graph = (
      await resource('queryCodingWorkflows', {
        operation: 'compile',
        input: {
          instance_id: randomUUID(),
          template: 'refactor',
          name: 'IU real requirement',
          goal: 'Read evidence.txt using ReadFile and return its exact text. Do not edit files or run tools other than reading the evidence.',
          constraints: 'Preserve evidence.txt exactly',
          scope_paths: ['evidence.txt'],
          evidence_paths: ['evidence.txt'],
          model: 'flash',
          max_iterations: 4,
          baseline: check('base'),
          checks: [check('final')],
          manual_checks: [{ id: 'human-real', description: spec.manual, required: true }],
        },
      })
    ).graph
    for (const n of graph.nodes)
      if (n.kind === 'CallLLM') {
        n.data.max_tokens = 512
        n.data.temperature = 0
      }
    file = 'requirement.blueprint'
    await client.saveBlueprint({ workspacePath, blueprint: await proto(graph), filePath: file })
    report.checks.push(
      'Compiled approved manual requirement into actual workflow implementation prompt',
    )
  } else {
    const message =
      spec.goal +
      '\nThis is a bounded synthetic acceptance task. First call QueryBlueprintNodes to discover the exact current kinds and signatures you need. Use DraftBlueprint compact JSON, not source files, DSL, WriteFile or invented UUIDs. Call DraftBlueprint with save=false first so the host can review the compiled draft, then request saving that same blueprint with save=true. Saving is authorized only through the resulting approval request. Do not request or start execution: the operator will execute the saved graph separately. Stop after saving. Do not spawn agents or start background tasks.'
    const options = {
      provider: 'openai-chat',
      model: 'flash',
      temperature: 0,
      max_tokens: 2048,
      max_iterations: 12,
      permission_mode: 'ask',
    }
    let previewSeen = false
    for await (const e of client.sendChat(
      { workspacePath, message, historyJson: '[]', optionsJson: JSON.stringify(options) },
      { timeoutMs: 300000 },
    )) {
      const detail = JSON.parse(e.detailJson || '{}')
      if (!e.kind.includes('reasoning'))
        report.events.push({ kind: e.kind, content: e.content, detail })
      if (
        e.kind === 'tool' &&
        detail.name === 'DraftBlueprint' &&
        detail.ok &&
        e.content.includes('Not saved (review only)')
      )
        previewSeen = true
      if (e.kind === 'approval') {
        const a = JSON.parse(e.content)
        let allow = false
        if (a.request_type === 'blueprint_save' && previewSeen) {
          safeGraph(a.blueprint)
          assert(a.path === `blueprints/${a.blueprint.id}.blueprint`)
          assert(a.content_digest.length === 64)
          graph = a.blueprint
          file = a.path
          allow = true
        }
        report.approvals.push({
          type: a.request_type,
          allow,
          path: a.path,
          digest: a.content_digest,
        })
        await client.respondApproval({
          workspacePath,
          requestId: detail.request_id,
          decision: allow ? 'AllowOnce' : 'DenyOnce',
        })
      }
      if (e.kind === 'error') throw Error(e.content)
    }
    assert(previewSeen, 'Model never produced a successful review-only draft')
    assert(graph && file, 'Model never saved through the bounded approval')
    const saved = JSON.parse(await readFile(join(workspacePath, file), 'utf8'))
    assert.equal(saved.id, graph.id)
    graph = saved
    const names = new Set(
      report.events.filter((e) => e.kind === 'tool_start').map((e) => e.detail.name),
    )
    assert(names.has('QueryBlueprintNodes'))
    assert(!names.has('RequestBlueprintRun') && !names.has('ExecuteBlueprint'))
    assert.equal((await client.listExecutions({ workspacePath })).executions.length, 0)
    report.checks.push(
      'Real model discovered current signatures, compiled a review-only draft and saved via separate approval; no automatic run',
    )
  }
  const before = await readFile(join(workspacePath, 'evidence.txt'))
  const confirmation =
    spec.id === 'manual_requirement'
      ? (
          await resource('queryCodingWorkflows', {
            operation: 'prepare_run',
            blueprint_id: graph.id,
          })
        ).digest
      : ''
  for await (const e of client.executeBlueprint(
    { workspacePath, blueprintId: graph.id, workflowConfirmation: confirmation },
    { timeoutMs: 160000 },
  )) {
    const detail = JSON.parse(e.detailJson || '{}')
    report.events.push({ kind: e.kind, message: e.message, detail })
    if (detail.run_id) report.run = detail.run_id
    if (e.kind === 'approval_request')
      await client.respondApproval({
        workspacePath,
        requestId: detail.request_id || e.message,
        decision: 'DenyOnce',
      })
    if (e.kind === 'error') throw Error(e.message)
  }
  const run = (await client.listExecutions({ workspacePath })).executions.find(
    (r) => r.blueprintId === graph.id,
  )
  assert(run, 'Actual execution missing')
  report.run = run.runId
  assert.equal(run.status, 'Completed')
  const snapshot = JSON.parse(run.dataJson)
  assert(
    JSON.stringify(snapshot.view.invocations).includes('IU_REAL_SENTINEL_中文'),
    'Actual node outputs did not contain source evidence',
  )
  assert.deepEqual(await readFile(join(workspacePath, 'evidence.txt')), before)
  report.output = await resource('queryCodingWorkflows', { operation: 'output', run_id: run.runId })
  if (spec.id !== 'catalog_read') {
    assert(
      report.output.nodes.some((n) =>
        n.messages.some((m) => m.role === 'tool' && m.actor === 'ReadFile' && m.detail?.ok),
      ),
    )
    assert(
      report.output.nodes.some((n) =>
        n.messages.some(
          (m) => m.role === 'assistant' && m.content.includes('IU_REAL_SENTINEL_中文'),
        ),
      ),
    )
  }
  if (spec.id === 'catalog_read')
    assert(
      graph.edges.some((e) => {
        const s = graph.nodes.find((n) => n.id === e.source_node),
          t = graph.nodes.find((n) => n.id === e.target_node)
        return (
          s.kind === 'Start' &&
          (t.kind === 'ReadFile' || t.data.tool_name === 'ReadFile') &&
          s.pins.find((p) => p.id === e.source_pin)?.pin_type === 'DataOutput'
        )
      }),
      'Missing Start-to-ReadFile data wire',
    )
  if (spec.id === 'catalog_llm_branch') {
    for (const k of ['Contains', 'Branch']) assert(graph.nodes.some((n) => n.kind === k))
    assert.equal(graph.nodes.filter((n) => n.kind === 'End').length, 2)
    const b = graph.nodes.find((n) => n.kind === 'Branch'),
      truePin = b.pins.find((p) => p.name === 'True')
    assert(
      snapshot.view.edges.some(
        (traversal) =>
          traversal.edge_id ===
          graph.edges.find((edge) => edge.source_node === b.id && edge.source_pin === truePin.id)?.id,
      ),
      'Actual run did not traverse True branch',
    )
  }
  report.blueprint_id = graph.id
  report.blueprint_path = file
  report.evidence_sha256 = createHash('sha256').update(before).digest('hex')
  report.checks.push(
    'Independently executed saved graph with real tool results; exact Unicode evidence preserved; persisted Output and branch/data wiring verified',
  )
  report.status = 'passed'
} catch (e) {
  report.status = 'failed'
  report.error = String(e)
  await client.abortChat({ workspacePath }, { timeoutMs: 5000 }).catch(() => {})
} finally {
  report.finished = new Date().toISOString()
  await writeFile(
    destination,
    JSON.stringify(report, (_, v) => (typeof v === 'bigint' ? String(v) : v), 2),
  )
  await client.closeWorkspace({ path: workspacePath }, { timeoutMs: 10000 }).catch(() => {})
  console.log(
    JSON.stringify({
      id: spec.id,
      status: report.status,
      error: report.error,
      checks: report.checks,
    }),
  )
  process.exit(report.status === 'passed' ? 0 : 1)
}
