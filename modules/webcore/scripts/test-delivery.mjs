import { test } from 'node:test'
import assert from 'node:assert/strict'
import { DeliveryAttempt } from '../src/core/delivery.ts'
const snapshot = () => ({
  workspace: 'workspace-a',
  session: '',
  newSession: crypto.randomUUID(),
  message: 'original\r\n中文',
  history: [],
  options: { model: 'current-model' },
})
function accepted(attempt, phase = 'started') {
  return {
    schema_version: 1,
    id: attempt.id,
    input_digest: 'fingerprint',
    session: attempt.snapshot.newSession,
    turn: crypto.randomUUID(),
    checkpoint: 'checkpoint',
    accepted_revision: 1,
    phase,
    outcome: null,
    accepted_ms: 1,
    updated_ms: 1,
  }
}
function transport(handler) {
  return {
    resourceRequest: async (_method, ws, payload) =>
      payload.operation === 'capabilities'
        ? { ok: true, data: { durable_manual_send: true, pending_reconciliation: true } }
        : payload.operation === 'reconcile_pending'
          ? { ok: true, data: { items: [], raw_copies_unchanged: true } }
          : handler(ws, payload),
  }
}

test('a model failure after durable acceptance does not make the same input unsent', async () => {
  let cleared = 0,
    queries = 0
  const attempt = new DeliveryAttempt(
    transport(() => {
      queries++
      throw Error('unexpected')
    }),
    snapshot(),
    () => cleared++,
  )
  assert.equal(
    await attempt.submit(async (request, onReceipt) => {
      assert.equal(request.options.delivery.id, attempt.id)
      assert.equal(request.options.delivery.new_session, request.newSession)
      onReceipt(accepted(attempt))
      return { ok: false, error: 'model failed after acceptance' }
    }),
    true,
  )
  assert.equal(cleared, 1)
  assert.equal(attempt.state, 'accepted')
  assert.equal(queries, 0)
  assert.equal(attempt.snapshot.message, 'original\r\n中文')
  assert.equal(
    await attempt.submit(async () => {
      throw Error('must not run twice')
    }),
    false,
  )
})

test('a missing read is followed by atomic retirement before input is declared unreceived', async () => {
  const calls = []
  let runs = 0
  const attempt = new DeliveryAttempt(
    transport(async (ws, payload) => {
      calls.push({ ws, payload })
      return {
        ok: true,
        data: {
          receipt:
            payload.operation === 'read'
              ? null
              : { ...accepted(attempt), phase: 'retired', accepted_revision: 0 },
        },
      }
    }),
    snapshot(),
  )
  assert.equal(
    await attempt.submit(async () => {
      runs++
      throw Error('network outcome unknown')
    }),
    false,
  )
  assert.equal(attempt.state, 'rejected')
  assert.equal(runs, 1)
  assert.deepEqual(
    calls.map((call) => call.payload.operation),
    ['read', 'retire'],
  )
  assert(calls.every((call) => call.payload.id === attempt.id && call.ws === 'workspace-a'))
})

test('when acceptance wins the retirement race its original receipt is kept', async () => {
  let callback = 0
  const attempt = new DeliveryAttempt(
    transport(async (_ws, payload) => ({
      ok: true,
      data: { receipt: payload.operation === 'read' ? null : accepted(attempt) },
    })),
    snapshot(),
    () => callback++,
  )
  assert.equal(await attempt.submit(async () => ({ ok: false, error: 'connection lost' })), true)
  assert.equal(attempt.state, 'accepted')
  assert.equal(callback, 1)
})

test('unknown reconciliation keeps the original ID and never automatically resends', async () => {
  const ids = []
  let connected = false,
    runs = 0,
    callback = 0
  const attempt = new DeliveryAttempt(
    transport(async (_ws, payload) => {
      ids.push(payload.id)
      return connected
        ? { ok: true, data: { receipt: accepted(attempt) } }
        : { ok: false, error: 'offline' }
    }),
    snapshot(),
    () => callback++,
  )
  assert.equal(
    await attempt.submit(async () => {
      runs++
      throw Error('offline')
    }),
    false,
  )
  assert.equal(attempt.state, 'unknown')
  assert.equal(
    await attempt.submit(async () => {
      runs++
      return { ok: true }
    }),
    false,
  )
  connected = true
  assert.equal(await attempt.reconcile(), true)
  assert(ids.every((id) => id === attempt.id))
  assert.equal(runs, 1)
  assert.equal(callback, 1)
})

test('an unsupported host or failed pending-draft backup cannot start a turn', async () => {
  let runs = 0
  const unsupported = new DeliveryAttempt(
    { resourceRequest: async () => ({ ok: false, error: 'Unimplemented' }) },
    snapshot(),
  )
  assert.equal(
    await unsupported.submit(async () => {
      runs++
      return { ok: true }
    }),
    false,
  )
  assert.equal(unsupported.state, 'unsupported')
  const backup = new DeliveryAttempt(
    transport(() => {
      throw Error('unused')
    }),
    snapshot(),
  )
  assert.equal(
    await backup.submit(
      async () => {
        runs++
        return { ok: true }
      },
      async () => false,
    ),
    false,
  )
  assert.equal(backup.state, 'draft')
  assert.equal(runs, 0)
})

test('recovered pending input only reconciles and rejects a receipt for another session', async () => {
  const saved = snapshot(),
    id = crypto.randomUUID()
  const attempt = new DeliveryAttempt(
    transport(async () => ({
      ok: true,
      data: { receipt: { ...accepted(attempt), session: 'different-session' } },
    })),
    saved,
    () => {
      throw Error('wrong target cleared')
    },
    () => {},
    id,
  )
  assert.equal(
    await attempt.submit(async () => {
      throw Error('recovered requests cannot auto-run')
    }),
    false,
  )
  assert.equal(await attempt.reconcile(), false)
  assert.equal(attempt.state, 'unknown')
  assert.equal(attempt.id, id)
})

test('duplicate submit calls during preflight still create at most one transport attempt', async () => {
  let release,
    runs = 0
  const gate = new Promise((resolve) => {
    release = resolve
  })
  const attempt = new DeliveryAttempt(
    transport(() => {
      throw Error('unused')
    }),
    snapshot(),
  )
  const first = attempt.submit(
    async (_request, onReceipt) => {
      runs++
      onReceipt(accepted(attempt))
      return { ok: true }
    },
    async () => {
      await gate
      return true
    },
  )
  assert.equal(
    await attempt.submit(async () => {
      runs++
      return { ok: true }
    }),
    false,
  )
  release()
  assert.equal(await first, true)
  assert.equal(runs, 1)
})

test('a host missing cross-reopen pending reconciliation cannot receive a protected send', async () => {
  const attempt = new DeliveryAttempt(
    { resourceRequest: async () => ({ ok: true, data: { durable_manual_send: true } }) },
    snapshot(),
  )
  assert.equal(
    await attempt.submit(async () => {
      throw Error('must not send to an older receipt host')
    }),
    false,
  )
  assert.equal(attempt.state, 'unsupported')
})

test('failure to reconcile host-backed pending inputs blocks a fresh send before backup or execution', async () => {
  const calls = []
  let backups = 0,
    runs = 0
  const attempt = new DeliveryAttempt(
    {
      resourceRequest: async (_method, _ws, payload) => {
        calls.push(payload.operation)
        return payload.operation === 'capabilities'
          ? { ok: true, data: { durable_manual_send: true, pending_reconciliation: true } }
          : { ok: false, error: 'pending receipt storage is unavailable' }
      },
    },
    snapshot(),
  )
  assert.equal(
    await attempt.submit(
      async () => {
        runs++
        return { ok: true }
      },
      async () => {
        backups++
        return true
      },
    ),
    false,
  )
  assert.deepEqual(calls, ['capabilities', 'reconcile_pending'])
  assert.equal(backups, 0)
  assert.equal(runs, 0)
  assert.equal(attempt.state, 'unknown')
})
