import { chromium, expect } from '@playwright/test'
import { writeFile } from 'node:fs/promises'
const output = process.argv[2],
  browser = await chromium.launch(),
  page = await browser.newPage({ viewport: { width: 1440, height: 1000 } })
const report = { checks: [], errors: [], requests: [] }
page.on('pageerror', (error) => report.errors.push(String(error)))
page.on('request', (r) => {
  if (/\/metteur.Daemon|\/api\//.test(r.url())) report.requests.push(r.url())
})
try {
  await page.goto('http://127.0.0.1:19120/src/previews/ide-repair/index.html?preview=IU-PV-01')
  await expect(page.getByRole('button', { name: 'Recovery copies', exact: true })).toBeVisible()
  await page.getByRole('button', { name: 'Expand Prompt', exact: true }).click()
  await page
    .getByRole('textbox', { name: 'Prompt expanded', exact: true })
    .fill('Long prompt\n中文换行 and preserved spaces  ')
  await page.getByRole('button', { name: 'Cancel', exact: true }).click()
  await expect(page.getByRole('textbox', { name: 'Prompt', exact: true })).not.toHaveValue(
    /Long prompt/,
  )
  await page.getByRole('button', { name: 'Expand Prompt', exact: true }).click()
  await page
    .getByRole('textbox', { name: 'Prompt expanded', exact: true })
    .fill('Long prompt\n中文换行 and preserved spaces  ')
  await page.getByRole('button', { name: 'Apply', exact: true }).click()
  await expect(page.getByRole('textbox', { name: 'Prompt', exact: true })).toHaveValue(
    'Long prompt中文换行 and preserved spaces  ',
  )
  // The collapsed native input only presents one line; the component model keeps newlines.
  await page.getByRole('button', { name: 'Expand Prompt', exact: true }).click()
  await expect(page.getByRole('textbox', { name: 'Prompt expanded', exact: true })).toHaveValue(
    'Long prompt\n中文换行 and preserved spaces  ',
  )
  await page.screenshot({ path: output + '.expanded.png' })
  await page.getByRole('button', { name: 'Cancel', exact: true }).click()
  report.checks.push('Expanded literal applies/cancels, retains multiline bytes and Chinese text')
  await page.screenshot({ path: output + '.editor.png' })
  await page
    .locator('.preview-controls')
    .getByRole('button', { name: 'Execution', exact: true })
    .click()
  await page.getByLabel('Run state', { exact: true }).selectOption('Stopped by you')
  await expect(page.getByText('Stopped by you', { exact: true }).last()).toBeVisible()
  await page.getByRole('button', { name: '3 steps · 4.1s', exact: true }).click()
  await page.screenshot({ path: output + '.output.png' })
  await page.locator('.preview-controls').getByRole('button', { name: 'Chat', exact: true }).click()
  await page.getByRole('button', { name: '4 steps · 8.2s', exact: true }).click()
  await expect(
    page.getByText(
      'Fixed normalization for blank input and preserved both callers. All four regression checks passed.',
    ),
  ).toBeVisible()
  await page.screenshot({ path: output + '.chat.png' })
  await page
    .locator('.preview-controls')
    .getByRole('button', { name: 'Memory', exact: true })
    .click()
  const list = page.getByRole('complementary', { name: 'Memory entries' })
  expect(await list.evaluate((e) => e.scrollHeight > e.clientHeight)).toBe(true)
  await list.evaluate((e) => {
    e.scrollTop = e.scrollHeight
  })
  await expect(list.getByRole('button', { name: 'Load more', exact: true })).toBeInViewport()
  report.checks.push(
    'Output/steps/stopped state and independently scrollable Memory are interactive fixture screens',
  )
  await page.getByRole('button', { name: 'Toggle theme', exact: true }).click()
  await page.setViewportSize({ width: 700, height: 950 })
  await page
    .locator('.preview-controls')
    .getByRole('button', { name: 'Editor', exact: true })
    .click()
  await page.screenshot({ path: output + '.narrow.png' })
  expect(report.requests).toHaveLength(0)
  expect(report.errors).toHaveLength(0)
} catch (error) {
  report.failure = String(error)
  await page.screenshot({ path: output + '.failure.png' })
}
await writeFile(output + '.json', JSON.stringify(report, null, 2))
await browser.close()
console.log(JSON.stringify(report))
if (report.failure || report.errors.length || report.requests.length) process.exitCode = 1
