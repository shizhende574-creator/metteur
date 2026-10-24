import { chromium, expect } from '@playwright/test'
import { readFile, writeFile } from 'node:fs/promises'
const [readyPath, workspace, output] = process.argv.slice(2)
const ready = JSON.parse(await readFile(readyPath, 'utf8')),
  browser = await chromium.launch(),
  page = await browser.newPage({ viewport: { width: 1440, height: 1000 }, serviceWorkers: 'block' })
const report = { checks: [], errors: [], effects: [], workspace },
  original = await readFile(workspace + '/toolbar.blueprint')
page.on('pageerror', (e) => report.errors.push(String(e)))
page.on('request', (r) => {
  if (/ExecuteBlueprint$|SaveBlueprint$|\/api\/chat\/stream/.test(r.url()))
    report.effects.push(r.url())
})
try {
  await page.goto(ready.baseURL + '/?open=' + encodeURIComponent(workspace))
  await page.waitForURL(/\/work\/[^/]+\//)
  await page.goto(
    ready.baseURL + '/work/' + new URL(page.url()).pathname.split('/')[2] + '/explorer',
  )
  await page
    .getByTestId('file-tree')
    .getByRole('button', { name: 'toolbar.blueprint', exact: true })
    .click()
  const pane = page.locator('[data-editor-pane]').first(),
    nodes = pane.locator('.vue-flow__node')
  await expect(nodes).toHaveCount(3)
  await pane.locator('div[tabindex="0"][aria-busy="false"]').first().focus()
  await page.keyboard.press('Control+a')
  await expect(pane.locator('.vue-flow__node.selected')).toHaveCount(3)
  await page.keyboard.press('Delete')
  await expect(nodes).toHaveCount(0)
  await page.keyboard.press('Control+z')
  await expect(nodes).toHaveCount(3)
  await page.keyboard.press('Control+Shift+z')
  await expect(nodes).toHaveCount(0)
  await page.keyboard.press('Control+z')
  await expect(nodes).toHaveCount(3)
  expect(await readFile(workspace + '/toolbar.blueprint')).toEqual(original)
  expect(report.effects).toHaveLength(0)
  expect(report.errors).toHaveLength(0)
  report.checks.push(
    'Canvas multi-delete, undo and redo work as one graph edit; final undo preserves the saved file with zero Save/Send/Run',
  )
} catch (e) {
  report.failure = String(e)
  await page.screenshot({ path: output + '.failure.png' })
}
await writeFile(output + '.json', JSON.stringify(report, null, 2))
await browser.close()
process.exit(report.failure || report.errors.length ? 1 : 0)
