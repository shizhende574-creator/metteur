# 输入恢复协议及当前接入边界

2026-10-22；当前正式接入文本、蓝图原始画布输入、Prompt 文档及主聊天/新聊天输入。监督和位置恢复仍待后续功能；生产完整链和 AC 尚未关闭。

主聊天在每窗口/实体使用独立输入副本；新聊天使用预分配 UUID，在宿主接收前不创建会话。每次 Send 另外保留点击时的固定输入；当前编辑稿与该原稿分别取得宿主 ACK 后才发起 Send。接收只清理匹配的正文/选择版本，拒绝不覆盖较新输入；未知保持原 pending ID，重开后仅对账。原稿保留到执行终态且源会话仍可读取；缺失来源保持可取回输入，不自动重建。恢复会重新核验 Prompt 当前权限及 revision/digest，不信任原 snapshot ID，不恢复 full/AllowOnce。

当前聊天/新聊天切换保留各自内存控制器，重开经 Recovery Copies 显式选择副本；新聊天原始稿可显式带入当前新草稿，已有会话的不同来源需打开对应会话或 Copy Text。未获 ACK 的本窗口输入可在恢复列表查看/导出；关闭页面的保护包含未备份聊天输入。没有持久浏览器正文缓存。监督当前仅改正共享 Composer 的提前清空/失败覆盖调用，尚未接入其独立持久恢复。

## 存储与访问

`Recovery(ResourceRequest) -> ResourceResponse` 是增量 RPC，使用现有 schema 1 envelope。action 支持 capabilities、list、read、backup、discard、export；scope 为 project/personal。owner 从宿主认证身份取得；project 绑定规范 workspace root，personal 使用用户数据库。客户端不能提交 owner/permissions。RPC 受现有方法 ACL 控制；恢复正文只是输入，使用资源、模型和工具必须按当前配置与权限重新校验。

namespace 为 `dw/recovery/<SHA256(scope, subject)>/`，位于既有 EXECUTION_STATE；使用 sync WAL 原子批写。恢复内容不进入 append-only 资源历史、不广播业务变化、不参与执行 checkpoint 或文件 rollback。没有新增 column family；旧程序可以打开原数据库，但不提供本能力。未知 marker/schema 不自动降级写入；损坏记录隔离，支持有界原始导出。

单记录序列化上限 1 MiB；每 owner/scope 64 MiB，活动记录最多 512，含 tombstone 总记录最多 4096。分页最多 50，只返回 metadata，原文需逐条 read。损坏记录占用配额；没有按年龄静默淘汰。达到上限保留现有副本和内存输入并返回失败。整库灾备回滚不在“已备份”保证内，应先导出最新恢复数据；业务 rewind 不影响本 namespace。

## 版本与回执

每窗口新建独立 window UUID，每实体有独立 draft UUID。backup 使用 expected revision、严格递增 edit_sequence、request UUID。宿主持久提交后才 ACK。最近完全相同的请求可重复取得原回执；同 ID 不同内容、旧 edit、跨窗口/目标和错误 CAS 均拒绝。更旧请求不会被当成新稿覆盖。

客户端 750ms debounce，持续编辑最长 5s 发起一次备份，至多一个在途写；期间输入合并到下一次。未知结果保留原 request/内容重试，ACK 只确认提交版本。断网后新增内容没有持久浏览器缓存，显示未确认/未备份；重连先对账。

保存成功仅条件清理已保存内容对应副本。discard 需宿主持久 tombstone；正文与选项清除，但身份和拒绝晚到写的凭据保留。丢弃回执未知时保留输入；继续输入不会被该旧回执清空，确认后转为新副本。页面卸载/切页只 dispose 内存控制器，不表示丢弃。当前关闭脏标签需明确确认并取得宿主 discard 回执；失败或确认期间的新输入阻止关闭。

宿主确认文件 rename 后，原 owner 的文本/画布关联按目录边界原子转移，保留 previous_entity，旧路径晚到写拒绝。转移超限时保留原副本并如实报告“文件已重命名，恢复副本未转移”。其他用户、外部 rename 或已删除源不会按名称猜关联。

## 恢复操作

列表按窗口和来源呈现独立副本；选择后先预览，显式 Use This Copy。当前脏输入先取得备份，异步期间新输入阻止替换；恢复 fork 新身份，保留原副本。基线与当前磁盘不一致进入冲突，不能自动覆盖。源缺失仍可导出，不自动重建会话/运行。恢复无效 JSON/front matter 保留原始字符串，不以正式保存的格式校验截断。

UI 沿用已批准 r2 的 IDE 布局，恢复栏只展示状态与必要入口；ID、窗口、revision 和长错误收进详情。正式功能必须另做生产链和最终体验验收，隔离预览 fixture 的成功不算耐久性证据。

## 验证入口

- L0：app/node 类型、定向 lint，增量 proto 生成物和全部 Rust 消费者构建。
- L1：`cargo test --locked -p metteur-daemon --lib storage::recovery::tests`；`cargo test --locked -p metteur-daemon --test dw_recovery`。覆盖真实 DB 重开、文件/RPC、容量、损坏、写失败、作用域、晚到写、rename 和有界通知。
- 客户端：`node --experimental-transform-types --test scripts/test-recovery.mjs`，覆盖备份在途/未知、错误、fork、discard 在途/未知的新输入。
- L2：`E2E_PORT=19084 pnpm exec playwright test e2e/daily-recovery.spec.ts --project=e2e`，fixture 只证明 UI/控制器连接；另复测受影响文档套件。
- 原始命令、退出码、源码状态、首轮失败和证据见 REVIEW_HANDOFF.md。L3/L4/L5 与聊天等剩余接入未完成，不能据此关闭 DW-02 或所有关联 AC。
