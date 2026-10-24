# IDE 修复交接

2026-10-24。用户14项反馈及追加的要求/人工修改备注传递缺口已整体实现、接入并完成本轮必要验收。两组r1经用户“确认两组 r1，按此接入”批准；实施前计划提交 `1fdb6f1`，产品提交 `fb17635d02fb8e15f6969bb55c5db61a14fd0b14`。后续收口提交仅更新验证辅助脚本与文档，不改变部署产品代码。

活动仓库 `F:/space/06_Projects/Agent/incoming/metteur-cloud-ff7b795-20261015/repo`，分支 `codex/ide-usability-20261024`；外层 Agent 根是另一仓库。仅本地提交，未推送、强推、写origin或创建/更新/合并PR，也未向其他聊天发消息。开工核对PR9实际已合并，旧“保持Draft”是历史状态；本轮没有合并它。fork/codex/nx-final-acceptance仍是本轮起点2fcfe205，任何后续推送需新授权并遵守fork快进边界。

## 正式运行与数据

入口 http://127.0.0.1:19103/。原生和前端都对应产品提交fb17635，加载冻结bin/web目录，保留旧哈希静态资产供已有页面正常更新。新证据根 `F:/space/06_Projects/Agent/ide-usability-evidence/20261024`：

- `formal/ready.json`：启动归属、路径和产品SHA；PID只代表记录时刻，接续先核对监听和实际可执行文件。
- `formal/artifact-manifest.json`：788个产品/辅助源码的LF哈希、451个Web文件及2个原生程序哈希。
- `formal/cold-backup.json`、`formal/cold-before-upgrade/`：停机冷备，141个文件逐项SHA一致，包含原数据、原配置与三个原工作区。
- `formal-before.json` 与 `.log`：以原f2057b5服务读取并与上次最终快照匹配，然后关闭工作区维护；provider调用0。
- `formal-after.json`、`formal-final-preservation.json`：本轮产品运行后的对照，均baselineMatched=true。原三个工作区及上次隔离surface工作区的聊天原始历史、ACK草稿/位置、任务预算/来源账本、运行状态、保存文件保持。新版界面检查仅使用新隔离surface工作区。
- `formal-surface.json`及PNG：正式模板图标打开四类真实模板，编辑工具栏图标打开原Recovery窗口；无多余顶部横栏，errors=[]，Send/Run/Save/Review/provider请求均0。
- `formal/serve-formal-runtime.mjs`：保留原配置、数据和workspace路径的启动器，不seed、不自动Send/Run。启动前先检查旧实例已停止；不要两个宿主同时打开原库。

本轮真实服务使用隔离data目录，验收完成后已停19121、19122及其自有宿主/受控provider。19120为隔离r1预览入口，文件已移至独立ide-repair目录，保留原usability历史入口源码。正式入口以19103为准。回退必须停本轮进程后恢复本轮整套冷备/配置/工作区，再使用上阶段冻结程序；不混用活动库和历史拷贝。没有另做破坏性冷恢复演练。

## 验收索引

映射和逐项边界见 [TASKS.md](TASKS.md)，使用见 [USAGE.md](USAGE.md)，模型见 [MODEL_REPORT.md](MODEL_REPORT.md)。主要回执：

| 范围 | 证据 |
|---|---|
| 图标、名称/类型、bool、画布多选复制、保存与撤销；多行编辑；完成状态隔离 | live-ui-interaction-first.json前三组成功，保留该文件后续失败 |
| 实时部分输出→主动停止、保留记录及取消状态隔离 | live-ui-cancel-retest.json成功组 |
| Memory滚动、Chat reasoning/tools同组、正文独立、审阅原草稿与显式发送 | live-ui-feedback-retest.json；sends=2均为脚本明确点击，runs=0，自动转交增量为0 |
| 空字符串、文字Ctrl+A、多行换行保护 | live-ui-shortcuts.json成功组 |
| 多选Delete/undo/redo | formal-selection.json，实际编辑后撤销回原图，零Save/Send/Run |
| 分栏Ctrl+S归属、画布全选隔离、Ctrl+Enter保留模板确认 | live-ui-split-retest.json，零Send/Run |
| 真实故障紧凑提示、Output原因、日志原始诊断 | live-ui-fault-final.json，复用同一已失败run，没有重复触发故障 |
| 真实模型编图与执行、重启/断连/恢复只读输出 | model-real-catalog-retest/、model-ui-offline.json |
| 宿主目录权限过滤和精确工具参数 | catalog-native-compile-retest.log（定向1项） |
| 要求编译、审阅快照与陈旧备注拒绝；Output身份与检查点不变 | feedback-output-host-assertion-retest.log（定向1项） |
| Unicode截断、首正文立即持久、关闭/重开、不触发工作区重载及不伪装检查点 | output-first-delta-test.log（定向1项） |
| 空字符串/false/null/中文多行画布与原生往返 | literal-regression-test.log（定向1项） |
| 必要静态/构建 | typecheck-final.log、lint-changed-receipt.log、helper-lint-retest.log、web-final-build.log、native-catalog-example-build.log |

静态检查与原生/Web构建通过；辅助脚本lint有1条no-shadow警告、无错误。只运行必要过滤项，没有重跑旧成功全矩阵或全仓库测试。Web包体提示沿用原构建，未为此次修复改包拆分。

## 失败与接受边界

首轮UI脚本的RPC编码、原生None/null、启动恢复/标签位移、导航大小写/可访问角色及fixture路由判断失败均保留；不会用脚本失败假称产品通过。联调中确实修复了Output依赖完成快照及首正文在思考后未即时写入的问题，取消复验通过。原始失败、错误定位、受影响复验分别记录，成功检查不无理由重跑。

真实模型独立账本30次结算，上界US$0.0935517；另1次故障脚本鉴权失败无usage，宿主0费用但成本完整性未确认，共31次外部尝试。401回执单列，不充当结算或零费用证明；之后已停止付费请求。不沿用旧阶段余额。下阶段需要明确新预算。

物理IME/S3、旧约6h03观察按此前人工接受边界继承，本轮未新增人工实测或长期观察。UI中文/快捷键、持久记录和断连通过不等于物理IME实测；三项真实合成样本不覆盖所有节点和长任务，也不支持任何模式普遍更优。新增128调用/2MiB记录上限不是完整压力基准。本阶段没有待开发或待接入的批准项；这些边界如实保留，不把已批准修复转交“下一阶段”。
