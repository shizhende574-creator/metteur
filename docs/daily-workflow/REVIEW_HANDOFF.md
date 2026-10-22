# 实施对话给规划复核对话的交接

更新：2026-10-22。活动分支 codex/daily-workflow-20261022；起点 6b19ffa36ea1013c8cd4eb14f49e5e32590081b1。本文件反映实际状态，不代替用户批准或完整验收。

## 当前进展

- 最新完整功能候选（基线2a536f8）：固定live-control目标，见LIVE_TARGETS.md。InterruptRequest新增target_kind/session_id/turn_id/run_id，AbortChatRequest新增session_id/turn_id；无目标/过期/串目标/停止中拒绝，蓝图Pause/Resume/Cancel不接受空run_id。CLI通过不等admission的control_target取得所有者，Web在await前快照目标；旧host能力不足时明确失败。TS生成物、真实/Mock网关、实际调用方和必要检查同功能配套。Queue/Steer仍无durable接收/消费回执，审批失败仍待修，不能关闭DW-03。
- 此功能检查：全workspace/all-targets check首轮两个旧smoke构造缺新字段失败，更新消费者后exit0（targets-workspace-check-first、targets-workspace-check-consumers-retest.log）；真实RPC最终11通过/exit0（targets-rpc-final.log），包括旧turn不能干扰同session新turn、错误chat通道不入蓝图、Stop后拒绝输入及控制查询不等admission。全workspace clippy首轮exit0，修正foreground测试模块位置新增警告后最终exit0，仅既有27项lib及1项旧diagnostics测试警告（targets-workspace-clippy-final.log）。
- 原生聊天烟测8通过、approval scopes RPC3通过、R-batch本地HTTP/审批/版本/插件1通过；R-batch改用实际CLI pause再复测1通过，蓝图精确pause/resume/cancel烟测1通过，均exit0（targets-smoke-chat-first、targets-approval-scopes-first、targets-r-batch-first、targets-r-batch-cli-retest、targets-smoke-blueprint-first.log）。L2网关/原聊天10通过，增加目标对象在await期间修改的窄复测1通过；受影响聊天/监督26通过。各轮重叠，不加总作AC。app类型/定向lint、正式前端构建exit0（targets-final-app-types、targets-final-lint、targets-production-final-build.log）。未更新生产运行版、不计L3，模型调用0，无push或合并。

- 最新增量为主聊天/监督分离与宿主前台占用快照，基线67e0578。主聊天不再按execution.active替换；状态/监督入口/直接Pause与Stop保持原IDE几何，Stop可越过在途Pause。实际契约与剩余旧Interrupt边界见FOREGROUND.md。新增只读检查不准备资源schema、不写checkpoint或业务通知；未知schema保守不可用。
- 本功能证据：RPC首轮8通过/1失败，取消蓝图按真实语义返回Aborted，测试错误地unwrap；修正为断言Aborted并确认槽位移除后9通过（foreground-rpc-first、foreground-rpc-cancel-retest.log）。追加只读构造后真实DB测试1通过（foreground-read-host-first.log），RPC9复测通过/exit0（foreground-rpc-read-retest.log）。daemon首次及只读构造最终clippy均仅既有27项lib警告/exit0（foreground-clippy-first、foreground-clippy-read-final.log）。app/node类型与定向lint exit0（foreground-final-*.log），lint首轮两处测试参数shadow已改。L2初始13通过；菜单/rewind复测15通过；Stop优先级5通过；监督/提案/Execution受影响27通过；A/B追加6通过。各轮有重叠，不相加作AC通过。所有原始日志foreground-*.log保留。
- 隔离预览适配构建、正式前端构建均exit0；foreground-isolated-build.log、foreground-production-final-build.log及对应目录。保留19082/19085冻结预览，没有将新产物冒充运行中的生产daemon。新Main截图在foreground-browser-ab-first内，已检查文字和布局，用户最终体验未代签。

- 最新完整功能：主聊天/新聊天的原始草稿、选择及 pending ID 接入已完成，源码候选 f60b614 加本功能差异；提交前摘要 composer-input-source-before-commit.json。同步主界面恢复栏/共享 Composer、source_available RPC、预览适配器、必要检查；完整提交后以 Git HEAD 对照摘要。本功能不关闭整个 DW-02/03。
- 当前检查：Windows RPC7/exit0（composer-source-rpc-first.log）；daemon clippy仅既有27项警告/exit0（composer-source-clippy-first.log）；客户端27/exit0（composer-controller-final.log），app/node类型及定向lint均exit0（composer-final-*.log）；L2聊天9/exit0（composer-browser-final.log），共享Composer/rewind/sequence/SSE/文档恢复18/exit0（composer-affected-first.log）。隔离预览适配构建及正式前端构建exit0（composer-isolated-build-first、composer-production-final-build.log）；尚未更新真实运行版，不算L3。
- 首轮失败保留：整合类型3处（未用ref/类型导出/异步窄化）修正后通过；lint未使用响应式版本读取改为响应式Map，测试参数重名已改；首轮L2的配置尚未ready，补测试前置等待；第二轮仅旧`.chat-thread`选择器错误，改实际`.chat-thread-inner`。没有改生产模型准入来满足夹具。早期6/8项与最终9项有重叠，不累加为AC。
- 主聊天当前可按原ID对账、保留较新输入并手工恢复宿主确认副本；新文字13～14px，技术信息在详情。余下监督持久输入、标签/位置、Queue/Steer固定目标、控制/审批、主监督分离及DW-04～09继续推进。最终用户体验验收、付费模型/数据授权、实际持续观察和Linux仍待完成，无付费调用、push或合并。

- DW-00 基线完成，提交 1b39ce8；G01～19、接口/容量/性能判定及最小复现见 BASELINE.md。复现发送拒绝清空、IME synthetic Enter 提交、rollback 失败不呈现、蓝图替换主聊天。简单切页原有 KeepAlive 保留输入，没有伪造该项失败。
- DW-01：r1 67f56d9 未批准。r2 5e40cbf 复用原 App/IDE，三组已获用户原文“批准三组 r2，保留现有 IDE 布局”。具体范围见 PREVIEWS.md。最终产品视觉签字尚未取得。
- DW-02 宿主/文档恢复已提交97436a6，主聊天接入见上述最新功能；监督/位置待接入。r3低密度隔离修订a8653cf，19085，原r2保留19082；r3未代填用户签字。手工发送协议9aa23cc，未决身份启动前对账f60b614均已完整本地提交。DW-02～09和AC-01～30均未关闭。
- 最新体验约束：“界面不要太多小字,信息量不要太大,要兼顾使用体验”。保留 IDE 几何；新增主要文字/操作约 13～14px，默认状态与必要操作，身份、revision、窗口和长诊断收进详情。已用于文档恢复栏和恢复弹窗，后续继续遵循。

## 当前功能与限制

以下保留9aa23cc/f60b614协议阶段的详细记录，其中尚未接入Composer描述的是当时边界；当前已接入范围以上述最新功能为准。

新增 pending_reconciliation 能力和 reconcile_pending：仅当前宿主 owner/project 的 chat/new_chat 未决身份先对账；未接收身份持久关闭、已接收保持原 turn，监督身份不混用，原始副本不改写。客户端在新发送前检查能力并对账，失败即保留输入且不启动新回合。基线9aa23cc；Windows真实RPC6、客户端9、SSE3、app类型及daemon clippy通过，仅既有27项lib警告；pending-delivery-*.log。此增量没有更新生产运行包，未接入Composer，不计AC关闭。


DW-03 接收协议完整提交 9aa23cc：增量 ChatDelivery RPC；SendChat.options.delivery 为可选精确 request ID/expected revision/新会话 UUID。普通旧请求保持原语义；新客户端先探测能力，不在旧服务静默回退受保护语义。接收把用户输入/session revision/receipt/关联新 task expected revision 一次 sync WAL 提交，再将 Started 持久化后进入原 runner。重复或重开只查询同一 ID，不新建执行；回执与模型结果分开。普通断流仍取消，获批 detached 沿用原 policy。

缺失查询不是明确未接收：同一 admission/resource gate 下 retire 未接收 ID；晚到发送拒绝，若接收已胜出则返回原回执。receipt 8192/每 scope16 MiB/单4 KiB，永久有界凭据不静默淘汰；达到上限拒绝新输入。第一次实际 UI 接入仍待完成，不能声称 Composer 已不丢输入。

本功能 Windows 检查：源码基线 a8653cf 加本功能完整差异；全 workspace/all-targets cargo check exit0（delivery-check-first.log）；存储首轮6通过，新增 retire 后8通过（delivery-host-first、delivery-host-retirement-retest.log）；RPC 首轮2通过/1失败，continuation样本10000 token低于真实根预算预留，合成预算改100000且max_tokens128后5通过（delivery-rpc-first、delivery-rpc-retirement-retest.log），未改预算守卫。客户端7通过（delivery-client-first.log），app类型exit0；SSE fixture和chat-sequence 3通过/exit0（delivery-browser-transport-first.log）；全workspace clippy exit0（delivery-clippy-first.log），发现一处新增可合并if后修正，窄复测仅既有27项lib警告/exit0（delivery-clippy-retest.log）。显式移除模型的宿主拒绝补强后 RPC5通过及窄clippy仅既有27警告/exit0（delivery-rpc-model-retest、delivery-clippy-model-retest.log）。未报告用量改为null（真实0仍为0），状态栏不显示虚构token数；新增流提前结束保留执行未知。定向计时/序列/回执5通过（delivery-usage-browser.log），终态三分支3通过（delivery-terminal-browser.log），有重叠，不累加作AC。app/node类型、定向lint通过，生产前端最终构建记录 manual-delivery-protocol-final-build.log。协议/客户端/检查同完整功能提交；模型费用0。

宿主 storage/recovery.rs 在 EXECUTION_STATE 使用独立 namespace，复用 durable sync 批写及 resource_gate；不进入资源历史、checkpoint 或业务通知。每记录 1 MiB、每 owner/scope 64 MiB、活动 512/含 tombstone 总 4096、metadata 分页 50。超过限制如实失败，无静默淘汰。支持 CAS、准确 request 重试、discard tombstone、rename 转移、损坏记录隔离及原始导出。增量 Recovery RPC 的生成物、Web 代理与测试实现同功能更新；旧服务明确 Unimplemented。

前端按窗口/实体隔离，750ms debounce/5s 最长发起，至多一个在途备份。未知重试原请求；旧 ACK 不清新输入。保存按对应内容清理，明确丢弃需宿主确认，丢弃回执晚到期间的新输入另存副本。文本/画布无效原始稿与 Prompt 草稿均保存原始输入；恢复先比较当前文件，只放进编辑器，源副本保留，不自动覆盖磁盘。无持久浏览器正文缓存。

限制：接口按 Recovery RPC ACL 和宿主 owner/scope 隔离；恢复输入不能赋予原模型/资源/工具权限，使用时仍需当前专用入口校验。每次备份在有界 scope 内扫描配额，最坏开销还需 DW-07 测量。位置、选项、未决发送身份尚无完整端到端接入。异步恢复/关闭新增 revision 守卫后，恢复与 workspace-reliability 共 13 项复测通过/exit 0，recovery-browser-final.log。本功能定向证据不能代替 L3 生产链、Linux、真实模型与最终体验验收。

## 检查与证据

证据根 F:/space/06_Projects/Agent/daily-workflow-evidence/20261022。源码起点 5e40cbf 加此完整功能差异；提交前保存源码摘要。Windows x64；没有付费 provider。

| 检查 | 实际结果与原始证据 |
|---|---|
| run_native.py cargo check --locked -p metteur-daemon --tests | exit 0；冷编译 32m25s，工具会话输出保留 |
| cargo test --locked -p metteur-daemon --lib storage::recovery::tests | 首轮 9 passed/exit 0，recovery-host-first.log；追加 rename 容量边界后 10 passed/exit 0，recovery-host-capacity-retest.log |
| cargo test --locked -p metteur-daemon --test dw_recovery | 首轮 3 passed/exit 0，recovery-rpc-first.log；Box 解析调整后 3 passed/exit 0，recovery-rpc-box-retest.log |
| cargo clippy --locked --workspace --all-targets | exit 0，recovery-protocol-clippy-first.log；发现新的大 enum 警告，改 Box 后 cargo clippy --locked -p metteur-daemon --lib exit 0，仅既有 27 项警告，recovery-clippy-box-retest.log |
| node --experimental-transform-types --test scripts/test-recovery.mjs | 原 6 passed；追加丢弃晚到/未知后新输入 2 项，8 passed/exit 0，recovery-client-discard-race.log |
| vue-tsc app、tsc node、定向 oxlint | exit 0；追加控制器竞态后 app 类型亦通过；定向 oxfmt 已整理 |
| Playwright workspace-reliability、blueprint-documents、file-write-boundaries | mock transport/0 retries/1 worker，16 passed/exit 0，editor-browser-retest.log |
| Playwright daily-recovery.spec.ts | L2 transport fixture/0 retries/1 worker，3 passed/exit 0，recovery-browser-third-retest.log；刷新恢复、两个窗口/丢弃、容量拒绝，不能证明 DB 耐久性 |
| vite build --outDir <证据根>/document-recovery-production | exit 0，document-recovery-production-build.log；已有 Monaco chunk/插件耗时提示；控制器和异步 UI 守卫修复后最终构建 exit 0，document-recovery-production-final-build.log，产物 document-recovery-production-final |

首轮失败均保留：旧文档 create-only 恢复被新增冲突守卫误拦，已修正；恢复 fixture connected=false 和 Monaco 自动配对改变输入分别修正测试前提；实际 Vue proxy structuredClone 抛错已改 JSON 数据克隆，保持 raw 原始字符串。r2 缺原 Tailwind source 的几何错误已在批准前修正并复查截图，原失败图保留。

## 运行和下一步

- 1801 旧复核服务实际未运行，R8 文件/DB 保留；5317～5319 属 Windows 排除端口，不改系统配置。
- 19080 开发 mock；19081 r1 静态隔离预览；19082 为已批准 r2 冻结静态产物。归属见 PREVIEWS.md。当前生产构建目录不是运行中的真实 daemon 实例。
- 收齐此功能定向复测，保存源码/产物摘要和完整本地功能提交；继续 DW-02 聊天/监督/位置及 DW-03 持久接收事务，然后依赖持续推进余下工作包。
- 可复核问题：namespace/配额、未知回执边界、未决输入接入现有 runner 的最小事务；三项产品决策不重新询问。

推进/推送原则：一个独立可验收功能一个完整本地提交，必要协议/客户端同属功能。保留历史，fork 顺序快进，不强推、不推 origin、不自动合并。新推送/发布依当前明确授权。本阶段持续推进，不逐包停等；具体视觉版本、付费模型数据/额度依实际门槛取得输入。无远端变更、模型调用 0。
