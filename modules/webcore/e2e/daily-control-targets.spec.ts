import { test, expect } from '@playwright/test'

// Actual gateway adapters against a typed transport fixture, not a daemon proof.
test('L2 gateway forwards exact immutable targets and refuses missing targets or old-host capability', async ({
  page,
}) => {
  await page.goto('/')
  const result = await page.evaluate(async () => {
    const path = '/src/core/grpc-gateway.ts',
      { GrpcGateway } = await import(path),
      gateway = new GrpcGateway('')
    const calls: any[] = []
    let supports = true
    gateway.client = {
      chatDelivery: async () => ({
        schemaVersion: 1,
        payloadJson: JSON.stringify({ targeted_controls: supports }),
        errorJson: '',
      }),
      sendInterrupt: async (request: any) => {
        calls.push({ method: 'interrupt', request })
        return {}
      },
      abortChat: async (request: any) => {
        calls.push({ method: 'stop', request })
        return {}
      },
    }
    const chat = { kind: 'chat', sessionId: 'original-session', turnId: 'original-turn' },
      blueprint = { kind: 'blueprint', runId: 'original-run' }
    const changingChat = { ...chat },
      pendingSend = gateway.sendInterrupt('workspace', 'raw\r\n input', 'Urgent', changingChat)
    changingChat.turnId = 'new turn during capability read'
    const sent = await pendingSend
    const changingStop = { ...chat },
      pendingStop = gateway.abortChat('workspace', changingStop)
    changingStop.sessionId = 'another session during capability read'
    const stopped = await pendingStop
    const directed = await gateway.sendInterrupt(
      'workspace',
      'urgent supervision',
      'Urgent',
      blueprint,
    )
    const missing = await gateway.sendInterrupt('workspace', 'ambiguous', 'Urgent')
    const unqualified = await gateway.abortChat('workspace')
    supports = false
    const oldHost = await gateway.sendInterrupt('workspace', 'no downgrade', 'Normal', chat)
    const oldStop = await gateway.abortChat('workspace', chat)
    return { sent, stopped, directed, missing, unqualified, oldHost, oldStop, calls }
  })
  expect(result.sent.ok && result.stopped.ok && result.directed.ok).toBe(true)
  expect(result.missing.ok || result.unqualified.ok || result.oldHost.ok || result.oldStop.ok).toBe(
    false,
  )
  expect(result.calls).toEqual([
    {
      method: 'interrupt',
      request: {
        workspacePath: 'workspace',
        message: 'raw\r\n input',
        priority: 'Urgent',
        targetKind: 'chat',
        sessionId: 'original-session',
        turnId: 'original-turn',
        runId: '',
      },
    },
    {
      method: 'stop',
      request: {
        workspacePath: 'workspace',
        sessionId: 'original-session',
        turnId: 'original-turn',
      },
    },
    {
      method: 'interrupt',
      request: {
        workspacePath: 'workspace',
        message: 'urgent supervision',
        priority: 'Urgent',
        targetKind: 'blueprint',
        sessionId: '',
        turnId: '',
        runId: 'original-run',
      },
    },
  ])
})
