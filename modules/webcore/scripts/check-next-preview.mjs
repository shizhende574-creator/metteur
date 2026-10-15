import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

const read = (path) => readFileSync(fileURLToPath(new URL(path, import.meta.url)), 'utf8')
const registry = JSON.parse(read('../src/previews/next/registry.json'))
assert.equal(registry.previews.length, 7)
assert.equal(new Set(registry.previews.map(p => p.id)).size, 7)
assert.equal(registry.approval, null)
assert.deepEqual(registry.integrationScope, [])
assert(!read('../src/router.ts').includes('previews/next'))
assert(!read('../src/main.ts').includes('previews/next'))
assert(read('../src/previews/next/index.html').includes("connect-src 'none'"))
assert(!read('../vite.next-preview.config.ts').includes('proxy:'))
console.log('NX preview registry and entry isolation: PASS (approval still pending)')
