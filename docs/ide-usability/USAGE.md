# IDE 修复后的使用约定

2026-10-24。正式入口 http://127.0.0.1:19103/，产品提交 `fb17635d02fb8e15f6969bb55c5db61a14fd0b14`。原 IDE 活动栏、标签、编辑/执行区域及主聊天/监督分工沿用既有设计。

## 编辑器与参数

Recovery Copies 的历史图标在当前编辑器工具栏；异常颜色、提示和 Draft Options 内的备份/丢弃仍保留。历史图标打开原恢复窗口。Chat 和蓝图的模板入口改为工具栏图标，提示和可访问名称仍为 Start from template。Chat 顶部不再常驻 Control & Input Receipts，原 Requests/回执机制继续生效。

蓝图 Ctrl/Cmd+S 保存拥有焦点的编辑器，包括字面量和 Inspector。画布 Ctrl/Cmd+A 全选节点，文字输入内仍全选文字；Ctrl/Cmd+C/V 复制/粘贴所选节点及内部连线，生成新的节点和 pin ID；Delete/Backspace 删除所选，Ctrl/Cmd+Z、Shift+Z/Y 撤销/重做。Ctrl/Cmd+Enter 沿用 Run 路径：模板图先确认冻结版本，普通图遵循既有独立运行行为。正在保存时 Run 禁用，快捷键同样不绕过保存守卫。

引脚名称、类型与字面量同时呈现，checkbox 左对齐。Inspector 输入框末尾的展开图标打开多行编辑，软换行不修改文本字节；Apply/Ctrl+Enter 应用，Cancel/Escape 放弃尚未应用的编辑。展开时 Ctrl/Cmd+S 应用并保存；之后 Cancel 不撤回已经保存的内容。含真实换行的折叠框只读，请展开编辑，防止单行控件抹掉换行。空字符串、false、零及结构化值按各自类型处理。composition 期间快捷键不抢占输入；本轮浏览器验证包含中文、焦点与保存，不冒称物理 IME 实测。

编辑器终态只属于相同蓝图和文件，使用 Last run / Saved version 标签区分历史执行与当前可编辑草稿。修改中的蓝图不冒用旧成功状态；其他蓝图不显示该运行的 finished/cancelled。

## Execution 与 Output

右侧 Supervision 默认 Output，另有 Chat、Requests、Reviews。Output 按真实 run、scope、frame、invocation、attempt 显示节点记录；使用相同的思考/工具 steps 和正文组件，支持查看工具结果与复制文本，不提供重试、编辑或自动发送。命令/普通节点结果来自持久 invocation；CallLLM 的文本、思考和工具事件另存，不用当前聊天替代节点输出。

流式输出有独立事件，不触发整个工作区重载。首段正文立即落盘，常规增量合并写入；结束、失败和取消保存已有部分。重开/刷新只读记录，断连明确标记并保留上次输出。停止显示 Stopped by you；真实错误仍显示 Run needs attention，点击打开 Execution log，Recorded run diagnostics 保留原始诊断。

记录限制为每文本字段16 KiB、每调用256行、每run128个调用和2 MiB，达到限制明确显示截断并保留先前记录，不补造完整内容。旧运行没有新增流式历史时仅展示可验证的冻结结果，不回填已丢失的思考/工具过程。此通道不替代检查点、执行恢复、权限或真实验收。

## ReAct 编图及要求传递

先用 QueryBlueprintNodes 搜索或按 kind 取得当前签名、默认值和示例；按页读取，单页最多20项，不一次灌入全部目录。目录来自 DraftBlueprint 使用的当前 registry，动态工具按当前角色过滤，查询不授予权限。

DraftBlueprint 使用 compact JSON：节点别名对应 `{kind: ...}`，参数直接作为节点键，flow 连接控制流，`target.pin <- source.pin` 或 `$alias.pin` 连接数据。Start 的自定义常量直接放在节点键上，会生成同名输出，不再额外包一层 data。编译报错应按实际签名修正。先 save=false 审阅，save=true 仍需保存审批；运行另行确认，不能以保存批准代替运行授权。工具别名在执行图中正常编译为 Tool/对应 tool_name。

四类模板的人工要求进入实现节点实际 Prompt。审阅后的 Prepare follow-up in Chat 取得宿主核实的完整审阅快照，附带 run、审阅 revision、evidence digest 和不可变引用；草稿列出未满足要求的 ID、说明、状态、人工决定及备注，附加的来源快照保留完整要求。原聊天及已有草稿保留，必须显式 Send；陈旧的文件、证据、备注或访问权使引用失效，需要刷新原审阅，不自动运行旧图或重置根预算。

Project Memory 的左列表和右正文在当前可用高度内独立滚动，分页按钮位于列表内。

## 验证与历史边界

逐项证据见 [TASKS.md](TASKS.md)，真实模型失败和费用见 [MODEL_REPORT.md](MODEL_REPORT.md)。不重跑上一阶段四类模板/三模式全矩阵，不据本轮三个合成样本宣称模型普遍可靠或某模式普遍更优。物理 IME、S3、旧长期观察的人工接受边界保持原样，本轮没有补称人工实测或新的长时观察。

新阶段模型额度必须重新明确授权；本轮余额不自动续用。正式服务保持原配置和原三个工作区，默认本地 fixture 的既有运行约定不变。回退前停止本轮服务，并整体恢复对应冷备数据/工作区/配置，不能混用同时打开的数据库；详细路径见 [REVIEW_HANDOFF.md](REVIEW_HANDOFF.md)。
