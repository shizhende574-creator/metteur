// Owned real Web/daemon with a scripted local provider. Never calls a paid endpoint.
import { createServer } from 'node:http'
import { createConnection } from 'node:net'
import { spawn } from 'node:child_process'
import { once } from 'node:events'
import { mkdir, writeFile, copyFile, readFile } from 'node:fs/promises'
import { createWriteStream } from 'node:fs'
import { join } from 'node:path'
const [root, staticDir, restart] = process.argv.slice(2)
await mkdir(root, { recursive: restart === '--restart' })
await mkdir(join(root, 'workspace'), { recursive: restart === '--restart' })
await writeFile(join(root, 'workspace/source.txt'), 'IU_SOURCE preserved 中文\n')
const receipts =
  restart === '--restart'
    ? JSON.parse(
        (await readFile(join(root, 'provider-receipts.json'), 'utf8')).replace(/^\uFEFF/, ''),
      )
    : { calls: 0, manualSeen: false, feedbackSeen: false, requests: [] }
const provider = createServer(async (req, res) => {
  if (req.url === '/fixture') {
    res.setHeader('content-type', 'application/json')
    res.end(JSON.stringify(receipts))
    return
  }
  if (req.url === '/close' && req.method === 'POST') {
    res.end('Closing owned fixture')
    setImmediate(() => void close())
    return
  }
  if (req.method !== 'POST') {
    res.writeHead(404).end()
    return
  }
  let raw = ''
  for await (const chunk of req) {
    raw += chunk
    if (raw.length > 2 * 1024 * 1024) throw Error('request too large')
  }
  const body = JSON.parse(raw),
    messages = body.messages ?? [],
    text = messages.map((m) => (typeof m.content === 'string' ? m.content : '')).join('\n')
  receipts.calls++
  receipts.manualSeen ||= text.includes('HUMAN_REQUIREMENT')
  receipts.feedbackSeen ||= text.includes('REVISION_NOTE')
  receipts.requests.push({
    stream: !!body.stream,
    hasManual: text.includes('HUMAN_REQUIREMENT'),
    tools: (body.tools ?? []).map((t) => t.function?.name),
  })
  if (text.includes('IU_FORCE_FAILURE')) {
    res.writeHead(503, { 'content-type': 'application/json' })
    res.end(
      JSON.stringify({ error: { message: 'IU_FORCE_FAILURE', type: 'local_fixture_failure' } }),
    )
    return
  }
  const feedback = text.includes('REVISION_NOTE'),
    cancel = !feedback && text.includes('IU_CANCEL'),
    tool = messages.some((m) => m.role === 'tool')
  const content = feedback
    ? 'IU_FEEDBACK_READBACK: recorded review note received after explicit Send.'
    : tool
      ? 'IU_NODE_FINAL: read source.txt, preserved 中文 and completed the scoped task.'
      : ''
  const call = {
    id: 'iu-read',
    type: 'function',
    function: { name: 'ReadFile', arguments: JSON.stringify({ path: 'source.txt' }) },
  }
  const usage = { prompt_tokens: 40, completion_tokens: 20, total_tokens: 60 }
  if (body.stream) {
    res.writeHead(200, { 'content-type': 'text/event-stream' })
    const delta = (value) =>
      res.write(
        `data: ${JSON.stringify({ choices: [{ index: 0, delta: value, finish_reason: null }] })}\n\n`,
      )
    delta({
      reasoning_content:
        'IU_THOUGHT: inspect the declared source and preserve its public behavior.',
    })
    if (cancel) {
      delta({ content: 'IU_PARTIAL: waiting for the controlled slow response.' })
      const timer = setTimeout(() => {
        delta({ content: ' finished' })
        res.end(
          `data: ${JSON.stringify({ choices: [{ index: 0, delta: {}, finish_reason: 'stop' }], usage })}\n\ndata: [DONE]\n\n`,
        )
      }, 12000)
      res.on('close', () => clearTimeout(timer))
      return
    }
    if (content) delta({ content })
    else delta({ tool_calls: [{ index: 0, ...call }] })
    res.end(
      `data: ${JSON.stringify({ choices: [{ index: 0, delta: {}, finish_reason: content ? 'stop' : 'tool_calls' }], usage })}\n\ndata: [DONE]\n\n`,
    )
  } else {
    res.setHeader('content-type', 'application/json')
    res.end(
      JSON.stringify({
        choices: [
          {
            message: {
              role: 'assistant',
              content: cancel ? 'IU_PARTIAL' : content,
              reasoning_content: 'IU_THOUGHT',
              ...(!content && !cancel ? { tool_calls: [call] } : {}),
            },
            finish_reason: content || cancel ? 'stop' : 'tool_calls',
          },
        ],
        usage,
      }),
    )
  }
})
provider.listen(0, '127.0.0.1')
await once(provider, 'listening')
const providerUrl = `http://127.0.0.1:${provider.address().port}`
await writeFile(
  join(root, 'config.toml'),
  `config_version = 2\n[daemon]\nautostart = "off"\nwake = false\n[llm]\ndefault_model = "fixture"\nmax_retries = 0\nproject_instructions = false\n[llm.models.fixture]\napi_type = "openai-chat"\napi_endpoint = "${providerUrl}"\napi_key = "local-fixture"\nmodel_id = "local-fixture"\ncontext_window_input = 64000\ncontext_window_output = 512\n[sandbox]\nenabled = true\nmode = "sandbox"\nwhitelist = [${JSON.stringify('node -e "process.exit(0)"')}]\n[oversight]\nmode = "off"\n`,
)
const target = 'D:/BaiduNetdiskDownload/metteur-a7-work/build/local-acceptance-native-target/debug',
  children = []
const runtimeBin = join(root, 'bin-' + Date.now())
await mkdir(runtimeBin)
for (const name of ['metteurd.exe', 'metteur-web.exe'])
  await copyFile(join(target, name), join(runtimeBin, name))
function start(name, args) {
  const log = createWriteStream(join(root, name + '.log'))
  const child = spawn(
    join(runtimeBin, name === 'daemon' ? 'metteurd.exe' : 'metteur-web.exe'),
    args,
    {
      windowsHide: true,
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  )
  child.stdout.pipe(log)
  child.stderr.pipe(log)
  children.push(child)
  return child
}
const daemon = start('daemon', [
  '--config',
  join(root, 'config.toml'),
  '--data-dir',
  join(root, 'data'),
  '--listen-addr',
  '127.0.0.1:19221',
])
async function listening(port) {
  for (let i = 0; i < 100; i++) {
    if (daemon.exitCode !== null) throw Error('Daemon exited')
    const ok = await new Promise((resolve) => {
      const s = createConnection({ host: '127.0.0.1', port })
      s.once('connect', () => {
        s.destroy()
        resolve(true)
      })
      s.once('error', () => resolve(false))
    })
    if (ok) return
    await new Promise((r) => setTimeout(r, 100))
  }
  throw Error('Listener timeout')
}
await listening(19221)
start('web', [
  '--daemon-addr',
  '127.0.0.1:19221',
  '--listen-addr',
  '127.0.0.1:19121',
  '--static-dir',
  staticDir,
])
await listening(19121)
const ready = {
  root,
  workspace: join(root, 'workspace'),
  baseURL: 'http://127.0.0.1:19121',
  providerUrl,
  daemonPid: children[0].pid,
  webPid: children[1].pid,
  fixture: true,
}
await writeFile(join(root, 'ready.json'), JSON.stringify(ready, null, 2))
console.log(JSON.stringify(ready))
async function close() {
  await writeFile(join(root, 'provider-receipts.json'), JSON.stringify(receipts, null, 2))
  for (const child of children.reverse()) child.kill()
  provider.close()
  process.exit()
}
process.on('SIGINT', () => void close())
process.on('SIGTERM', () => void close())
