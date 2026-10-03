import { test, expect } from '@playwright/test'

test('presence overrides survive form saves, TOML and reset without pinning inherited values', async ({ page }) => {
  await page.goto('/')
  await page.getByLabel('Workspace path').fill('D:/metteur-demo/metrics')
  await page.getByRole('button', { name: 'Open', exact: true }).click()
  await expect(page.getByTestId('file-tree')).toBeVisible()
  const result = await page.evaluate(async () => {
    const corePath = '/src/core/index.ts', storePath = '/src/stores/config.ts', tomlPath = '/src/lib/toml.ts'
    const { gateway } = await import(corePath)
    const { useConfigStore } = await import(storePath)
    const { configToToml, tryParseToml } = await import(tomlPath)
    await gateway.setConfig({ config_version: 2, sandbox: { enabled: true }, llm: { thinking_budget_tokens: 4096, default_model: 'global' } })
    const store = useConfigStore()
    await store.load()
    store.ws = { config_version: 2, sandbox: { enabled: false }, llm: { thinking_budget_tokens: 0 } }
    const before = JSON.parse(JSON.stringify(store.effective))
    const error = await store.save('workspace')
    const after = JSON.parse(JSON.stringify(store.effective))
    const toml = configToToml(store.ws)
    const roundtrip = tryParseToml(toml)
    store.resetKey('workspace', 'llm', 'thinking_budget_tokens')
    store.resetKey('workspace', 'sandbox', 'enabled')
    const resetError = await store.save('workspace')
    return { before, after, error, resetError, roundtrip, reset: store.effective, raw: store.ws, toml }
  })
  expect(result.error).toBe('')
  expect(result.resetError).toBe('')
  expect(result.before).toEqual(result.after)
  expect(result.after.sandbox.enabled).toBe(false)
  expect(result.after.llm.thinking_budget_tokens).toBe(0)
  expect(result.after.llm.default_model).toBe('global')
  expect(result.roundtrip.sandbox.enabled).toBe(false)
  expect(result.roundtrip.llm.thinking_budget_tokens).toBe(0)
  expect(result.toml).not.toContain('default_model')
  expect(result.reset.sandbox.enabled).toBe(true)
  expect(result.reset.llm.thinking_budget_tokens).toBe(4096)
  expect(result.raw.llm.thinking_budget_tokens).toBeUndefined()
})
