# IDE 修复隔离预览

2026-10-24。用户明确回复“确认两组 r1，按此接入”，IU-PV-01/02 r1已批准。预览只用 fixture，无真实文件、Send、Run或付费模型；批准后进入真实实现与联调，不以fixture代替验收。

- IU-PV-01：[编辑器、工具栏、参数与长文本](http://127.0.0.1:19120/src/previews/ide-repair/index.html?preview=IU-PV-01)。顶部图标、名称/类型/字面量同显、bool左对齐、长文本展开/应用/取消、Memory独立滚动。
- IU-PV-02：[Output、停止状态与Chat steps](http://127.0.0.1:19120/src/previews/ide-repair/index.html?preview=IU-PV-02)。Output/Chat/Requests/Reviews切换、节点输出、思考工具同组及独立正文、取消中性状态；主会话与监督不串线。

顶部fixture控制可切换Editor/Chat/Execution/Memory及主题；执行状态下拉仅为预览场景控制，不是产品控件。源码为 `modules/webcore/src/previews/ide-repair`；复用LiteralTextEditor，保持原App IDE壳。应用类型检查通过；preview-r1-first记录脚本误点原App Chat导航而离开预览的首次失败，限定fixture工具栏后preview-r1-locator-retest通过，errors=[]/requests=[]，包含展开/取消/原多行字节、Output、Chat正文与步骤、Memory末项和窄屏明暗截图。不代替真实服务或物理IME。

沿用原 SPEC §8.1.12 和用户本轮“需要预览的界面先给我确认”：用户查看具体 preview_id/revision并确认后接入生产。后端/独立检查在等待期间继续。
