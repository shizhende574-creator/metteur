import { chromium, expect } from '@playwright/test'
import { readFile, writeFile } from 'node:fs/promises'
const [readyPath, output, baseURL = 'http://127.0.0.1:19122'] = process.argv.slice(2),
  ready = JSON.parse(await readFile(readyPath, 'utf8'))
const browser = await chromium.launch(),
  page = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
    serviceWorkers: 'block',
  }),
  report = { checks: [], errors: [], sends: 0, runs: 0, workspace: ready.workspace, baseURL }
page.on('pageerror', (e) => report.errors.push(String(e)))
page.on('request', (r) => {
  if (/ExecuteBlueprint$/.test(r.url())) report.runs++
  if (/\/api\/chat\/stream/.test(r.url())) report.sends++
})
try {
  await page.goto(baseURL + '/?open=' + encodeURIComponent(ready.workspace))
  await page.waitForURL(/\/work\/[^/]+\//)
  await page
    .getByRole('navigation', { name: 'Main navigation' })
    .getByRole('button', { name: 'Execution', exact: true })
    .click()
  const panel = page.getByRole('region', { name: 'Node output' })
  await expect(panel).toContainText('IU_REAL_SENTINEL_中文', { timeout: 15000 })
  await expect(panel).toContainText('CallLLM')
  await expect(panel.getByRole('button', { name: 'Retry', exact: true })).toHaveCount(0)
  await page.reload()
  await expect(panel).toContainText('IU_REAL_SENTINEL_中文')
  await page.context().setOffline(true)
  await expect(panel).toContainText('Disconnected', { timeout: 15000 })
  await expect(panel).toContainText('IU_REAL_SENTINEL_中文')
  await page.context().setOffline(false)
  await page.reload()
  await expect(panel).toContainText('IU_REAL_SENTINEL_中文')
  await page.screenshot({ path: output + '.png' })
  expect(report.sends).toBe(0)
  expect(report.runs).toBe(0)
  expect(report.errors).toHaveLength(0)
  report.checks.push(
    'Production browser reads persisted real-model output after host restart, offline and reconnect/reload; cached output remains explicitly disconnected and read-only with zero Send/Run',
  )
} catch (e) {
  report.failure = String(e)
  await writeFile(output + '.page.txt', await page.locator('body').innerText())
  await page.screenshot({ path: output + '.failure.png' })
}
await writeFile(output + '.json', JSON.stringify(report, null, 2))
await browser.close()
process.exit(report.failure || report.errors.length ? 1 : 0)
