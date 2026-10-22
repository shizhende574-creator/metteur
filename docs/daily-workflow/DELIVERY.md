# 手工发送的持久接收协议

DW-03 第一个独立功能，2026-10-22。包括 daemon 接收/查询、Web SSE 适配及独立客户端状态机。Composer、未决输入恢复、Queue/Steer、控制与 IME 仍待后续接入；定向通过不代表整个工作包或 AC 已关闭。

先查询增量 ChatDelivery RPC 的 capabilities。新请求在既有 SendChat.options_json 提供 `delivery: { id, expected_revision, new_session? }`，身份为非 nil UUID。new_session 在显式 Send 接受后才成为新会话。旧请求保持原语义和统一准入；旧服务返回 Unimplemented，新客户端不声称旧服务有新保证；mock 不提供伪耐久回执。durable_queue=false 如实表达其边界。

接收前核验准入、session/revision、Prompt 选择和模型配置，捕获原 paired checkpoint。原始用户输入/context、transcript、单调 session revision、回执及关联新 continuation task 的 expected revision 一次 sync WAL 提交。再将 Started 持久化后进入原 ReAct runner。不新增执行器或普通聊天 detached 语义。拒绝不增加用户消息或 session revision。

回执保存输入指纹、session/turn/checkpoint、accepted revision、phase 和有界 outcome，不重复保存正文。指纹含原 session/message/history/options 精确内容。相同 ID/内容返回原回执；异内容拒绝。重复只产生 delivery/delivery_replay，不第二次调用模型/工具、不伪造新用量。Accepted 是输入接收；之后模型失败仍是已接收输入的执行结果。

客户端未知只按原 ID 对账，不自动重传或新建 ID。恢复未决身份只对账，不恢复 full/AllowOnce 权限。**一次 read 不存在不是明确未接收**，原请求仍可能晚到。retire 与 accept 使用相同 admission/resource gate：未接受则写永久 Retired 凭据、拒绝晚到发送；接受已胜出则返回原回执，不取消或替换执行。只有 Retired 才能归为未接收、允许用户新提交；retire 结果未知时仍保留原 ID。

接收后启动前崩溃留下 Accepted；Started 的效果保守视为未知。本实现不自动重放任一种窗口。普通断流继续取消，只有已授权 detached policy 延续。文件/context rewind 不回退 ledger。整库灾备回滚可能回退凭据，需保留最新数据库/回执；不能据缺失进程推断从未产生效果。

存储为 EXECUTION_STATE 独立 namespace `dw/delivery/<SHA256(canonical workspace, authenticated subject)>/`，schema1。单回执4KiB，每 owner/workspace 8192条/16MiB，分页50；永久轻量凭据不静默淘汰，满额拒绝新输入。原 message+history+options 上限1MiB。未知 schema/损坏回执 fail closed，不当作不存在。已删除会话 revision 不归零，新聊天不复用已存在/已删除身份。owner 来自认证连接。

Windows 存储8、真实 RPC/原 mock runner5、客户端状态机7、SSE与原序列3项定向通过。首轮 RPC 合成 continuation 样本预算过小被真实守卫拒绝；修正样本预算及 max_tokens 后通过，没有放松守卫。命令、源码候选、首次失败与未测项见 REVIEW_HANDOFF.md 和工作区 daily-workflow-evidence/20261022。生产完整故事、Linux、真实模型、用户使用及剩余入口未验收。
