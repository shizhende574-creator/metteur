import { chromium, expect } from '@playwright/test'
import { readFile, writeFile } from 'node:fs/promises'
const [readyPath, priorPath, output] = process.argv.slice(2),
  ready = JSON.parse(await readFile(readyPath, 'utf8')),
  prior = JSON.parse(await readFile(priorPath, 'utf8')),
  root = prior.workspace
const browser = await chromium.launch(),
  page = await browser.newPage({ viewport: { width: 1600, height: 1000 }, serviceWorkers: 'block' })
const report = { checks: [], errors: [], runs: 0, sends: 0, workspace: root }
page.on('pageerror', (e) => report.errors.push(String(e)))
page.on('request', (r) => {
  if (/ExecuteBlueprint$/.test(r.url())) report.runs++
  if (/\/api\/chat\/stream/.test(r.url())) report.sends++
})
try {
  await page.goto(ready.baseURL + '/?open=' + encodeURIComponent(root))
  await page.waitForURL(/\/work\/[^/]+\//)
  await page.goto(
    ready.baseURL + '/work/' + new URL(page.url()).pathname.split('/')[2] + '/explorer',
  )
  const tree = page.getByTestId('file-tree')
  await expect(tree).toBeVisible()
  await tree.getByRole('button', { name: 'iu-output.blueprint', exact: true }).click()
  const a = JSON.parse(await readFile(root + '/iu-output.blueprint', 'utf8')),
    llm = a.nodes.find((n) => (n.type ?? n.kind) === 'CallLLM')
  if (process.argv[5] !== 'split-only') {
    await page.locator(`.vue-flow__node[data-id="${llm.id}"]`).click()
    await page.getByRole('button', { name: 'Expand Prompt', exact: true }).click()
    const expanded = page.getByRole('textbox', { name: 'Prompt expanded', exact: true }),
      original = await expanded.inputValue()
    await expanded.press('Control+a')
    expect(await expanded.evaluate((e) => e.selectionEnd - e.selectionStart)).toBe(original.length)
    await expanded.fill('')
    await expanded.press('Control+s')
    await expect
      .poll(async () => {
        const g = JSON.parse(await readFile(root + '/iu-output.blueprint', 'utf8')),
          n = g.nodes.find((node) => node.id === llm.id),
          p = (n.inputs ?? n.pins).find((pin) => pin.name === 'Prompt')
        return n.values?.[p.id] ?? n.data?.Prompt ?? n.data?.prompt
      })
      .toBe('')
    await expanded.fill(original)
    await expanded.press('Control+s')
    await page.getByRole('button', { name: 'Cancel', exact: true }).click()
    await expect(page.getByRole('textbox', { name: 'Prompt', exact: true }).last()).toHaveAttribute(
      'readonly',
      '',
    )
    report.checks.push(
      'Native text Ctrl+A remains text selection; empty string really saves; expanded Ctrl+S preserves multiline text; collapsed multiline field prevents newline loss',
    )
  }
  await tree
    .getByRole('button', { name: 'iu-independent.blueprint', exact: true })
    .click({ button: 'right' })
  await page.getByText('Open in Split View', { exact: true }).click()
  await expect(page.locator('[data-editor-pane]')).toHaveCount(2)
  const panes = page.locator('[data-editor-pane]'),
    right = panes.nth(1),
    flag = right.locator('input[type=checkbox]').first()
  await expect(flag).toBeVisible()
  const originalFlag = await flag.isChecked(),
    leftBytes = await readFile(root + '/iu-output.blueprint')
  await flag.setChecked(!originalFlag)
  await flag.press('Control+s')
  await expect
    .poll(async () => {
      const g = JSON.parse(await readFile(root + '/iu-independent.blueprint', 'utf8')),
        n = g.nodes.find((node) => (node.type ?? node.kind) === 'Not'),
        p = (n.inputs ?? n.pins).find((pin) => pin.name === 'A')
      return n.values?.[p.id] ?? String(n.data?.A ?? n.data?.a)
    })
    .toBe(String(!originalFlag))
  expect(await readFile(root + '/iu-output.blueprint')).toEqual(leftBytes)
  await flag.setChecked(originalFlag)
  await flag.press('Control+s')
  await expect(right.getByRole('button', { name: 'Run', exact: true })).toBeEnabled()
  const leftSelection = await panes
    .first()
    .locator('.vue-flow__node.selected')
    .evaluateAll((nodes) => nodes.map((n) => n.getAttribute('data-id')))
  const canvas = right.locator('div[tabindex="0"][aria-busy="false"]').first()
  await canvas.focus()
  await page.keyboard.press('Control+a')
  await expect(right.locator('.vue-flow__node.selected')).toHaveCount(3)
  expect(
    await panes
      .first()
      .locator('.vue-flow__node.selected')
      .evaluateAll((nodes) => nodes.map((n) => n.getAttribute('data-id'))),
  ).toEqual(leftSelection)
  report.checks.push(
    'Split focus routes Ctrl+S to the right blueprint only; canvas Ctrl+A selects that graph without changing the left selection',
  )
  await expect(panes.first().getByRole('button', { name: 'Run', exact: true })).toBeEnabled()
  await panes.first().locator('div[tabindex="0"][aria-busy="false"]').first().focus()
  await page.keyboard.press('Control+Enter')
  await expect(
    page.getByRole('button', { name: 'Run confirmed version', exact: true }),
  ).toBeVisible()
  await page.getByRole('button', { name: 'Cancel', exact: true }).click()
  expect(report.runs).toBe(0)
  expect(report.sends).toBe(0)
  report.checks.push(
    'Ctrl+Enter follows the existing workflow confirmation and does not automatically execute',
  )
  await page.screenshot({ path: output + '.png' })
  expect(report.errors).toHaveLength(0)
} catch (e) {
  report.failure = String(e)
  await writeFile(output + '.page.txt', await page.locator('body').innerText())
  await page.screenshot({ path: output + '.failure.png' })
}
await writeFile(output + '.json', JSON.stringify(report, null, 2))
await browser.close()
console.log(JSON.stringify(report))
process.exit(report.failure || report.errors.length ? 1 : 0)
