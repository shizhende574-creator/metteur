# IDE 修复任务与验收

2026-10-24。完整授权、边界和预算见 [PLAN.md](PLAN.md)。证据根 `F:/space/06_Projects/Agent/ide-usability-evidence/20261024`。状态：PLANNED → IN_PROGRESS → IMPLEMENTED → VERIFIED；具体界面未批准标 WAITING_PREVIEW，不能以预览代替真实服务。

| ID | 用户反馈/完整交付包 | 状态 | 验收标准/证据 |
|---|---|---|---|
| IU-00 | 基线、计划/任务、独立预算 | VERIFIED（文档） | 本地/远端2fcfe205、工作树初始干净；服务离线、PR9已合并；独立US$1/40授权记录 |
| IU-01 | #1/#7/#10 工具栏与入口 | IMPLEMENTED（定向验收已完成） | 无额外Recovery/Receipts/Template横栏；图标可访问；恢复/异常/模板仍可用 |
| IU-02 | #2 蓝图快捷键 | IMPLEMENTED（定向验收已完成） | 画布/字面量/Inspector/分栏分别保存；文字与节点全选分派，多选复制删除/undo一致 |
| IU-03 | #3 Output事件/持久化/呈现 | IMPLEMENTED（定向验收已完成） | 文本/工具/命令/失败真实可见；run/frame/attempt不串；刷新、断连、取消及上限如实 |
| IU-04 | #4 运行状态归属 | IMPLEMENTED（定向验收已完成） | A完成/取消后B不串；修改版本不冒用旧成功；其他图占用/控制目标正确 |
| IU-05 | #5/#6/#13 参数和引脚 | IMPLEMENTED（定向验收已完成） | 冻结字面量可见；普通exec隐藏内部标签、分支保留；名称/类型/值同显；bool左对齐及false持久 |
| IU-06 | #8 真实目录与编图工具 | IMPLEMENTED（定向验收已完成） | 按需目录/签名含动态Tool；错误诊断可修正；真实模型生成并实际执行，审批/权限/预算不绕过 |
| IU-07 | #9 Memory滚动 | IMPLEMENTED（定向验收已完成） | 多条目左列表可达末项/Load more；右侧长正文可读，窄屏正常 |
| IU-08 | #11 主动停止状态 | IMPLEMENTED（定向验收已完成） | 中性紧凑状态；真实故障仍明确；原始原因可查 |
| IU-09 | #12 Chat steps合并 | IMPLEMENTED（定向验收已完成） | 纯reasoning/工具同组，正文独立；无工具思考/流式/刷新/失败顺序正确 |
| IU-10 | #14 长文本展开 | IMPLEMENTED（定向验收已完成） | 展开/换行/应用/取消/焦点/IME正确，原文本及保存重开一致 |
| IU-11 | 要求与人工修改备注完整传递 | IMPLEMENTED（定向验收已完成） | 人工要求进入实际模型请求；已验证审阅快照进入原草稿，ID/备注/revision/digest完整；陈旧及跨scope拒绝，零自动Send/Run |
| IU-12 | 两组隔离预览及用户确认 | VERIFIED（预览范围） | 用户“确认两组 r1，按此接入”；类型/定向交互与明暗窄屏通过，fixture零请求；生产另验 |
| IU-13 | 集成、真实模型、兼容和完整收口 | IN_PROGRESS（正式切换/交接） | 必要检查/真实服务/US$1-40账本、失败保留、所有14+缺口验收、使用/交接/运行与数据/文档同步 |

## 逐项映射与必要检查

用户1→IU01；2→IU02；3→IU03；4→IU04；5/6/13→IU05；7/10→IU01；8→IU06；9→IU07；11→IU08；12→IU09；14→IU10；追加传递缺口→IU11。新增项目预设、通用子图复跑不在本次授权范围。

共享节点参数/执行身份集中正反例；输出与工具目录各做必要宿主契约；界面合并真实服务场景，失败仅定向复验。未执行项/失败/人工接受分别记录，不能仅据任务表勾选宣布完成。

## 实施验收进度

两组 r1 已批准并接入。真实服务回执 `live-ui-interaction-first.json`（3组成功复用）、`live-ui-cancel-retest.json`（取消成功复用）、`live-ui-feedback-retest.json`（Memory/Chat/反馈显式发送）、`live-ui-shortcuts.json`（空字符串/多行成功复用）、`live-ui-split-retest.json`（分栏/运行快捷键）、`live-ui-fault-final.json`（真实503与原始日志）、`model-ui-readback.json`（真实模型输出重开）。其余失败回执保留，不合并伪装首轮通过。

真实模型三项冻结样本在 `model-real-catalog-retest/summary.json` 通过。独立账本30次已结算，上界US$0.0935517；另有一次故障脚本未配置模型名触发401，无provider usage，宿主0费用但tokensComplete/costComplete=false，详见 `provider-auth-rejection.json`，共31次外部尝试，停止后不再调用。正式版本/数据冷备/收口文档尚在完成，不能据此提前标记 IU13 完成。
