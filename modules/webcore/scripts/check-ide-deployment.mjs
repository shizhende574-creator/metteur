import { chromium, expect } from '@playwright/test'
import { readFile, writeFile, mkdir, copyFile } from 'node:fs/promises'
const [readyPath, output, source] = process.argv.slice(2),
  ready = JSON.parse(await readFile(readyPath, 'utf8')),
  workspace = readyPath.replace(/\/ready.json$/, '') + '/surface-workspace'
await mkdir(workspace, { recursive: true })
await copyFile(source, workspace + '/toolbar.blueprint')
const browser = await chromium.launch(),
  page = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
    serviceWorkers: 'block',
  }),
  report = { baseURL: ready.baseURL, workspace, checks: [], errors: [], writes: [] }
page.on('pageerror', (e) => report.errors.push(String(e)))
page.on('request', (r) => {
  if (/ExecuteBlueprint$|SaveBlueprint$|RecordWorkflowReview$|\/api\/chat\/stream/.test(r.url()))
    report.writes.push(r.url())
})
try {
  await page.goto(ready.baseURL + '/?open=' + encodeURIComponent(workspace))
  await page.waitForURL(/\/work\/[^/]+\//)
  const nav = page.getByRole('navigation', { name: 'Main navigation' })
  await nav.getByRole('button', { name: 'Chat', exact: true }).click()
  await expect(page.getByText('Control & Input Receipts', { exact: true })).toHaveCount(0)
  await expect(page.getByText('Start from template', { exact: true })).toHaveCount(0)
  await page.getByRole('button', { name: 'Start from template', exact: true }).click()
  for (const name of ['Fix a bug', 'Add tests', 'Refactor safely', 'Add a small feature'])
    await expect(page.getByRole('button', { name: new RegExp(name) })).toBeVisible()
  await nav.getByRole('button', { name: 'Explorer', exact: true }).click()
  await page
    .getByTestId('file-tree')
    .getByRole('button', { name: 'toolbar.blueprint', exact: true })
    .click()
  await page.getByRole('button', { name: 'Recovery Copies', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'Recovery Copies', exact: true })
  await expect(dialog).toBeVisible()
  await dialog.getByRole('button', { name: 'Close', exact: true }).click()
  await page.screenshot({ path: output + '.png' })
  expect(report.writes).toHaveLength(0)
  expect(report.errors).toHaveLength(0)
  const usage = await (await fetch(ready.providerUrl + '/fixture')).json()
  expect(usage.calls).toBe(0)
  report.provider_calls = usage.calls
  report.checks.push(
    'Formal frozen assets open the real four-template catalog via icon and Recovery Copies via editor toolbar icon; no legacy top strips, Send/Run/Save/Review or provider request',
  )
} catch (e) {
  report.failure = String(e)
  await writeFile(output + '.page.txt', await page.locator('body').innerText())
  await page.screenshot({ path: output + '.failure.png' })
}
await writeFile(output + '.json', JSON.stringify(report, null, 2))
await browser.close()
process.exit(report.failure || report.errors.length ? 1 : 0)
