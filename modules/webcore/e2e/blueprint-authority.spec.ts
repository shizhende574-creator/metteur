import { test, expect } from '@playwright/test'

test('save and execution transmit the same canvas file representation', async ({ page }) => {
  await page.goto('/')
  const result = await page.evaluate(async () => {
    const path = '/src/core/grpc-gateway.ts'
    const { GrpcGateway } = await import(path)
    const gateway = new GrpcGateway('http://unused')
    const calls: Record<string, { filePath?: string; fileJson?: string; blueprintJson?: string }> = {}
    gateway.client = {
      saveBlueprint: async (request: object) => { calls.save = request },
      executeBlueprint: async function* (request: object) { calls.run = request; yield* [] },
    }
    const graph = { id: 'graph', name: 'Canvas', entryNodeId: 'node', edges: [], nodes: [{
      id: 'node', type: 'Start', nodeType: 'Event', title: 'My entry', position: { x: 2, y: 3 },
      inputs: [], outputs: [], data: {}, values: {},
    }] }
    const saved = await gateway.saveBlueprint('workspace', graph, 'plans/main.blueprint')
    const ran = await gateway.executeBlueprint('workspace', graph.id, () => {}, graph)
    return { saved, ran, path: calls.save.filePath, file: JSON.parse(calls.save.fileJson ?? ''),
      inline: JSON.parse(calls.run.blueprintJson ?? '') }
  })
  expect(result.saved.ok).toBe(true)
  expect(result.ran.ok).toBe(true)
  expect(result.path).toBe('plans/main.blueprint')
  expect(result.inline).toEqual(result.file)
  expect(result.inline.nodes[0]).toMatchObject({ title: 'My entry', position: { x: 2, y: 3 } })
})

test('model blueprint approval displays the proposed contents and base file', async ({ page }) => {
  await page.goto('/')
  await page.getByLabel('Workspace path').fill('D:/metteur-demo/metrics')
  await page.getByRole('button', { name: 'Open', exact: true }).click()
  await expect(page.getByTestId('file-tree')).toBeVisible()
  await page.evaluate(async () => {
    const path = '/src/stores/chat.ts'
    const { useChatStore } = await import(path)
    useChatStore().approval = { requestId: 'proposal', detail: JSON.stringify({
      request_type: 'blueprint_save', source: 'model_draft', path: 'plans/main.blueprint',
      base: { snapshot_id: 'snapshot', blueprint_uri: 'plans/main.blueprint', blob_hash: 'original-content' },
      blueprint: { name: 'Review this plan', nodes: [{ kind: 'ReadFile', data: { path: 'report.txt' } }] },
    }) }
  })
  const dialog = page.getByRole('dialog')
  await expect(dialog).toContainText('Approve blueprint save')
  await expect(dialog).toContainText('plans/main.blueprint')
  await expect(dialog).toContainText('original-content')
  await expect(dialog).toContainText('report.txt')
  await expect(dialog.getByRole('button', { name: 'Allow', exact: true })).toBeVisible()
})
