# AC-01～30 当前验收矩阵

2026-10-23。宿主候选`ccecd9c`、已批准前端`65cc92f`，当前正式入口19103，清单`s4-ui-integrated-r1/manifest.json`；原生清单保留`s4-lifecycle-runtime-r1/manifest.json`；未受影响证据分别复用9212d6a（投递/控制）、190b48d（文件）及64d464f（结果），后续eeb930c事件、ccecd9c关闭/断流增量定向验收。L1宿主/真实DB、L2组件与受控响应、L3生产链、L4真实模型、L5平台/持续观察分别报告。

**技术通过**表示表列实现和行为证据齐备，不代替最终人类签字。**待外部/持续项**保留原范围；当前没有把全阶段标完成。完整原AC文本在根NEXT_STAGE_PLAN.md，本表不缩减其要求。证据根为工作区外`daily-workflow-evidence/20261022`；详细首轮失败及窄复测见REVIEW_HANDOFF.md和各契约页。

剩余四项的唯一人工清单见[MANUAL_ACCEPTANCE.md](MANUAL_ACCEPTANCE.md)：AC-19/28复用当前观察并自动汇总；用户只做真实IME及一次实际睡眠。专用样本已准备，不重走已通过故事；睡眠需等待就绪标记以免破坏8h窗口。

| AC | 当前结果 | 证据与适用范围 |
|---|---|---|
| 01 | 技术通过 | `s4-session-ownership-first`：A交接选B、占用Enter不提交、完成不抢页、主Chat不变监督，B正文及位置恢复。 |
| 02 | 技术通过 | RECOVERY/VIEW_STATE/SUPERVISION_INPUTS的四类输入与位置L1/L2；生产双窗口文本、主Chat B恢复；冷副本四类原文保持。 |
| 03 | 技术通过 | `s4-recovery-two-window-narrow`真实ACK/重开/磁盘变化先冲突；实际daemon退出后的ACK及冷恢复；未ACK/写失败L1/L2。 |
| 04 | 技术通过 | WORKFLOW_RECEIPTS/输入控制器的拒绝/队列失效/未知及选择恢复；DW03生产输入；本候选gRPC/SSE断流不再执行后续工具。 |
| 05 | 技术通过 | ACK固定编辑版本、晚到/新编辑/多窗口CAS的L1/L2；本候选两真实窗口各保留原稿，恢复不覆盖另一副本。 |
| 06 | 技术通过 | Send/Queue/Steer/监督/临时输入/审批/控制原ID及异内容拒绝；ccecd9c生产断流后同ID只返回原receipt，未重放。 |
| 07 | 技术通过 | FILES.md及DW04生产链：脏稿选已保存版本、摘要/BOM/CRLF、后来工具读盘另版本；资源/恢复重新核验。 |
| 08 | 技术通过 | DW04真实再次变化CAS拒绝、Keep Both/create-only及恢复；S4双窗口磁盘变化不覆盖，A故事人工/Agent双方保留。 |
| 09 | 技术通过 | 文件/聊天rewind、rollback失败/受影响范围/原稿保护的L1/L2；Linux当前chat_rewind，恢复namespace及Stop账本不回退。 |
| 10 | 技术通过 | RESULTS/LIFECYCLE：实际pending approval原因、固定来源/直接run、共享结果及无source新计划；执行/检查/人工判断分列。 |
| 11 | 技术通过 | NX launch/manifest/mirror/保存失败及幂等L1；S3精确确认只保留结果；S4真实A/B交接，旧确认/未知效果守卫保留。 |
| 12 | 技术通过 | S3只保留结果零自动回合；S4单Job无原观察流时同会话单次dispatch；两阶段同根任务；停止/失败分支宿主契约。 |
| 13 | 技术通过 | Windows/Linux原因矩阵及当前权限/期限/预算；实际未过期daemon恢复effects_unknown、Resume拒绝、零新模型请求和不变消费。 |
| 14 | 技术通过 | nx_final_boundaries新用户输入栅栏；S3显式带有来源的结果Send；S4两次交接保留事件/历史/根预算，新提案只追加稿。 |
| 15 | 技术通过 | 固定target/revision、未知先对账和晚到守卫L1/L2；生产待审批时Stop保留原文件；两次实际daemon退出不复活旧来源。 |
| 16 | 技术通过 | VIEW_STATE的历史深链/缺失源/晚回复L1/L2；生产B新页面重开及独立恢复；缺失源RPC不重建会话。 |
| 17 | 技术通过 | S4故事A同名Personal/Project明确快照、有文件绑定的model_opinion及run来源；资源/约定权限和失效L1/L2，检查不取模型自述。 |
| 18 | 技术通过 | EVENTS.md：一窗口一工作区共享流、ACL/取消/重连；S3重复通知/Usage/Reviews；生产10秒无相关1s/1.5s查询。 |
| 19 | 待持续项 | 当前冻结候选固定样本达标，见PERFORMANCE.md；8小时观察进行中，记录实际样本/JS及native内存/采样间隙，不提前关闭。 |
| 20 | 技术通过 | 两完整故事L3及Job/child/timer/blueprint原生证据已齐；获批当前候选L4三任务首轮通过，14/36请求、费用估算/实际金额未知分开，见REAL_MODEL_ACCEPTANCE.md。 |
| 21 | 技术通过 | r2及UI-CONSISTENCY r1均获明确批准；用户反馈“人工使用暂时没问题”。明暗/窄屏/键盘及正式19103保真/真实控件首轮通过，见UI_CONSISTENCY.md。 |
| 22 | 技术通过 | 冻结源码/产物零差异；Windows真实R8前进升级/冷回退/返回当前及后续修改保全；四类恢复冷副本逐文件哈希；兼容限制已说明。 |
| 23 | 待人类 | 合成IME/keyCode229、换行/重复及占用键盘路径L2；生产占用Enter保留B稿；实际Windows IME候选确认尚待用户操作。 |
| 24 | 技术通过 | 无效画布/文本/Prompt/聊天原文、源删除/rename/晚到写/tombstone的L1/L2；冷恢复无效原稿、生产显式恢复与保留另一窗口。 |
| 25 | 技术通过 | Windows/Linux存储11项：durable故障、配额、损坏/未知schema隔离、1000次有界/零业务通知；rewind隔离；RESTORE_GUIDE边界明确。 |
| 26 | 技术通过 | 全普通入口统一准入、旧/无目标控制拒绝、能力不足不降级L1/L2；生产A运行时B不能Send；旧R8缺Recovery明确Unimplemented。 |
| 27 | 待人类项 | 普通断流两桥、获批detached、页面/工作区关闭、Job/child收尾及真实daemon退出恢复已验；实际OS睡眠/唤醒未执行，不以时钟模拟冒充。 |
| 28 | 待持续项 | 原生1000次同稿两业务行/零业务通知及配额、客户端合并；真实双窗口保护已验；新候选双窗口长期样本正在记录。 |
| 29 | 技术通过 | 恢复顺序/重复选择、旧Prompt/文件/模型/权限重新校验L1/L2及DW04/S3生产Send；冷副本不恢复权限，个人scope隔离。 |
| 30 | 技术通过 | 当前冷恢复真实直接run无Continuation关联；S3/S4聊天启动绑定精确A会话，保存/版本/准入及直接控制共享宿主守卫。 |

当前26项技术证据齐备，待关闭为AC-19/23/27/28，不能只靠已有测试数改成通过。新US$1/最多36请求方案已获批准，三任务L4已首轮通过，实际14请求；历史US$5未复用。用户已反馈当前人工使用暂时没问题，并批准UI r1；实际时长、IME与睡眠/唤醒未明示，不代签。

当前已知回退限制：R8不能原地列出新增dw namespace的运行数据；安全回退需升级前冷副本并保全新稿和后续修改，已实测，不能称原地降级兼容。执行结果详情按现有有界策略呈现；需要完整执行细节进入原run，不把截取内容或模型意见视为人工验收。
