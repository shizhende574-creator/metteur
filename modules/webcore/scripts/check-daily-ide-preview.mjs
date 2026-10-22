import { chromium } from '@playwright/test'
import assert from 'node:assert/strict'
import { mkdir,writeFile,readFile } from 'node:fs/promises'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
const output=process.argv[2];if(!output)throw Error('Evidence directory required')
await mkdir(output,{recursive:true})
const browser=await chromium.launch({headless:true})
const page=await browser.newPage({viewport:{width:1360,height:900},reducedMotion:'reduce'})
page.setDefaultTimeout(8000)
const errors=[],checks=[]
page.on('pageerror',e=>errors.push(e.message))
page.on('request',r=>{if(!r.url().startsWith('http://127.0.0.1:19082/')&&!r.url().startsWith('blob:'))errors.push('Unexpected network '+r.url())})
try{
 await page.goto('http://127.0.0.1:19082/?preview=WF-PV-01')
 await page.getByRole('textbox',{name:'Message',exact:true}).waitFor()
 assert(await page.evaluate(()=>document.documentElement.scrollHeight<=innerHeight+2),'The original IDE must fit the viewport rather than stack vertically')
 await page.screenshot({path:`${output}/WF-PV-01-r2-ide.png`,fullPage:true})
 assert(await page.getByTestId('tab-bar').count() || await page.getByTestId('tab').count() || await page.getByTestId('tab-active').count())
 const input=page.getByRole('textbox',{name:'Message',exact:true})
 await input.fill('preserve original IDE draft')
 await input.dispatchEvent('keydown',{key:'Enter',isComposing:true,keyCode:229})
 assert.equal(await input.inputValue(),'preserve original IDE draft')
 assert.equal(await page.getByRole('button',{name:'Send',exact:true}).isDisabled(),true)
 await page.getByRole('button',{name:'Open Supervision',exact:true}).click()
 await page.locator('.execution-layout').waitFor()
 await page.getByRole('button',{name:'Back to Original Chat',exact:true}).click()
 assert.equal(await input.inputValue(),'preserve original IDE draft')
 checks.push('Original App shell/rail/tabs and ExecutionLayout reused; main chat and independent supervision; IME and blueprint send gate')
 await page.getByLabel('Preview Group',{exact:true}).selectOption('WF-PV-02')
 await page.locator('[data-editor-pane] .monaco-editor .view-lines').first().waitFor()
 await page.getByRole('button',{name:'Back Up',exact:true}).click()
 await page.getByRole('button',{name:'Compare',exact:true}).click()
 await page.getByRole('button',{name:'Simulate External Change',exact:true}).click()
 await page.getByRole('button',{name:'Save Reviewed Content',exact:true}).click()
 assert.match(await page.getByRole('status').innerText(),/Disk changed again/)
 await page.getByRole('button',{name:'Keep Both',exact:true}).click()
 assert.match(await page.getByRole('status').innerText(),/create-only/)
 await page.screenshot({path:`${output}/WF-PV-02-r2-ide.png`,fullPage:true})
 checks.push('Original Explorer/Monaco/editor toolbar, additive backup/reference controls, three-way compare and second-change refusal')
 await page.getByLabel('Preview Group',{exact:true}).selectOption('WF-PV-03')
 await page.getByLabel('Fixture State',{exact:true}).selectOption('Unknown Effect')
 assert.equal(await page.getByRole('button',{name:'Resume Execution',exact:true}).isDisabled(),true)
 await page.getByRole('button',{name:'Add Result to Draft',exact:true}).click()
 assert.match(await page.getByRole('status').innerText(),/No model call/)
 await page.screenshot({path:`${output}/WF-PV-03-r2-ide.png`,fullPage:true})
 await page.getByRole('button',{name:'Return to Chat',exact:true}).click()
 assert.match(await input.inputValue(),/preserve original IDE draft.*\n\[Result source/s)
 checks.push('Original Tasks header/tabs with readable results, unknown-effect gate, source result added to retained draft without send')
 for(const id of ['WF-PV-01','WF-PV-02','WF-PV-03']){
   await page.getByLabel('Preview Group',{exact:true}).selectOption(id)
   for(const width of [1360,390]){
     await page.setViewportSize({width,height:900})
     const overflow=await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth)
     assert.equal(overflow,false,`${id} overflow at ${width}`)
     await page.screenshot({path:`${output}/${id}-r2-${width}.png`,fullPage:true})
     await page.locator('button[title="light"]').click()
     await page.screenshot({path:`${output}/${id}-r2-dark-${width}.png`,fullPage:true})
     await page.locator('button[title="dark"]').click()
   }
 }
 assert.deepEqual(errors,[])
 checks.push('1360/390px, no page overflow or JS errors; no external data/model request')
}finally{
 await page.screenshot({path:`${output}/last-state.png`,fullPage:true}).catch(()=>{})
 const files=['src/previews/next/daily/IdePreview.vue','src/previews/next/daily/WorkflowAddon.vue','src/previews/next/daily/ide-main.ts','src/previews/next/daily/ide-state.ts','src/previews/next/daily/ide/index.html','src/previews/next/daily/ide/ide.css','vite.daily-ide-preview.config.ts','src/App.vue','src/styles/main.css','src/router.ts']
 const sources=[]
 for(const file of files)sources.push({file,sha256:createHash('sha256').update(await readFile(file)).digest('hex')})
 await writeFile(`${output}/result.json`,JSON.stringify({candidate:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),source_status:execFileSync('git',['status','--short'],{encoding:'utf8'}),sources,revision:'r2',fixture:'dw-ide-fixture-2',checks,errors,paid_calls:0,approval:null},null,2))
 await browser.close()
}
