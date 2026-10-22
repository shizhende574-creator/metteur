# 实施对话给规划复核对话的交接

更新：2026-10-22。活动分支 codex/daily-workflow-20261022；起点 6b19ffa36ea1013c8cd4eb14f49e5e32590081b1。本文件反映实际状态，不代替用户批准或完整验收。

## 当前进展

- DW-00 基线完成，提交 1b39ce8；G01～19、接口/容量/性能判定及最小复现见 BASELINE.md。复现发送拒绝清空、IME synthetic Enter 提交、rollback 失败不呈现、蓝图替换主聊天。简单切页原有 KeepAlive 保留输入，没有伪造该项失败。
- DW-01：r1 67f56d9 未批准。r2 5e40cbf 复用原 App/IDE，三组已获用户原文“批准三组 r2，保留现有 IDE 布局”。具体范围见 PREVIEWS.md。最终产品视觉签字尚未取得。
- DW-02 宿主和文档恢复正在收尾验证；聊天、监督和位置恢复尚未接入。当前未提交差异属本实施对话。DW-03～09 未关闭，AC-01～30 尚未关闭。
- 最新体验约束：“界面不要太多小字,信息量不要太大,要兼顾使用体验”。保留 IDE 几何；新增主要文字/操作约 13～14px，默认状态与必要操作，身份、revision、窗口和长诊断收进详情。已用于文档恢复栏和恢复弹窗，后续继续遵循。

## 当前功能与限制

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
