# IDE 修复任务与验收

2026-10-24。完整授权、边界和预算见 [PLAN.md](PLAN.md)。证据根 `F:/space/06_Projects/Agent/ide-usability-evidence/20261024`。状态：PLANNED → IN_PROGRESS → IMPLEMENTED → VERIFIED；具体界面未批准标 WAITING_PREVIEW，不能以预览代替真实服务。

| ID | 用户反馈/完整交付包 | 状态 | 验收标准/证据 |
|---|---|---|---|
| IU-00 | 基线、计划/任务、独立预算 | VERIFIED（文档） | 本地/远端2fcfe205、工作树初始干净；服务离线、PR9已合并；独立US$1/40授权记录 |
| IU-01 | #1/#7/#10 工具栏与入口 | VERIFIED（见回执） | 无额外Recovery/Receipts/Template横栏；图标可访问；恢复/异常/模板仍可用 |
| IU-02 | #2 蓝图快捷键 | VERIFIED（见回执） | 画布/字面量/Inspector/分栏分别保存；文字与节点全选分派，多选复制删除/undo一致 |
| IU-03 | #3 Output事件/持久化/呈现 | VERIFIED（见回执） | 文本/工具/命令/失败真实可见；run/frame/attempt不串；刷新、断连、取消及上限如实 |
| IU-04 | #4 运行状态归属 | VERIFIED（见回执） | A完成/取消后B不串；修改版本不冒用旧成功；其他图占用/控制目标正确 |
| IU-05 | #5/#6/#13 参数和引脚 | VERIFIED（见回执） | 冻结字面量可见；普通exec隐藏内部标签、分支保留；名称/类型/值同显；bool左对齐及false持久 |
| IU-06 | #8 真实目录与编图工具 | VERIFIED（见回执） | 按需目录/签名含动态Tool；错误诊断可修正；真实模型生成并实际执行，审批/权限/预算不绕过 |
| IU-07 | #9 Memory滚动 | VERIFIED（见回执） | 多条目左列表可达末项/Load more；右侧长正文可读，窄屏正常 |
| IU-08 | #11 主动停止状态 | VERIFIED（见回执） | 中性紧凑状态；真实故障仍明确；原始原因可查 |
| IU-09 | #12 Chat steps合并 | VERIFIED（见回执） | 纯reasoning/工具同组，正文独立；无工具思考/流式/刷新/失败顺序正确 |
| IU-10 | #14 长文本展开 | VERIFIED（见回执） | 展开/换行/应用/取消/焦点/IME正确，原文本及保存重开一致 |
| IU-11 | 要求与人工修改备注完整传递 | VERIFIED（见回执） | 人工要求进入实际模型请求；已验证审阅快照进入原草稿，ID/备注/revision/digest完整；陈旧及跨scope拒绝，零自动Send/Run |
| IU-12 | 两组隔离预览及用户确认 | VERIFIED（预览范围） | 用户“确认两组 r1，按此接入”；类型/定向交互与明暗窄屏通过，fixture零请求；生产另验 |
| IU-13 | 集成、真实模型、兼容和完整收口 | VERIFIED（正式切换/交接） | 必要检查/真实服务/US$1-40账本、失败保留、所有14+缺口验收、使用/交接/运行与数据/文档同步 |

## 逐项映射与必要检查

用户1→IU01；2→IU02；3→IU03；4→IU04；5/6/13→IU05；7/10→IU01；8→IU06；9→IU07；11→IU08；12→IU09；14→IU10；追加传递缺口→IU11。新增项目预设、通用子图复跑不在本次授权范围。

共享节点参数/执行身份集中正反例；输出与工具目录各做必要宿主契约；界面合并真实服务场景，失败仅定向复验。未执行项/失败/人工接受分别记录，不能仅据任务表勾选宣布完成。

## 实施验收进度

两组 r1 已批准并接入。真实服务回执 `live-ui-interaction-first.json`（3组成功复用）、`live-ui-cancel-retest.json`（取消成功复用）、`live-ui-feedback-retest.json`（Memory/Chat/反馈显式发送）、`live-ui-shortcuts.json`（空字符串/多行成功复用）、`live-ui-split-retest.json`（分栏/运行快捷键）、`live-ui-fault-final.json`（真实503与原始日志）、`model-ui-readback.json`（真实模型输出重开）。其余失败回执保留，不合并伪装首轮通过。

真实模型三项冻结样本在 `model-real-catalog-retest/summary.json` 通过。独立账本30次已结算，上界US$0.0935517；另有一次故障脚本未配置模型名触发401，无provider usage，宿主0费用但tokensComplete/costComplete=false，详见 `provider-auth-rejection.json`，共31次外部尝试，停止后不再调用。正式切换、冷备、前后及最终对照、使用/模型/交接文档已同步；IU13按下列实际回执收口。

## 收口与适用边界

正式产品fb17635，19103。141文件冷备逐项匹配，原三个工作区及上一隔离surface数据前后/最终快照baselineMatched=true；`formal-surface.json`真实入口无Send/Run/Save/Review/provider请求。使用、界面及历史边界见 [USAGE.md](USAGE.md)，模型首轮/复验与401计数见 [MODEL_REPORT.md](MODEL_REPORT.md)，完整回执/构建/运行/回退见 [REVIEW_HANDOFF.md](REVIEW_HANDOFF.md)。

IU10的VERIFIED指浏览器中文/软换行/焦点/快捷键及存储往返，不是新增物理IME人工验收。IU03包含真实模型持久输出、重启、断连重连与取消，Unicode截断有定向单元证据；128调用/2MiB不是完整压力测试。沿用既有物理IME/S3/观察时长的人工接受边界，无新增承诺或后推开发项。未重跑旧全矩阵；首轮失败与脚本失败全部保留。

`formal-selection.json`补齐受影响的多选Delete、撤销和重做路径：真实画布3→0→3→0→3，保存文件不变，零Save/Send/Run；不重跑已成功的复制/粘贴或分栏检查。

## 用户追加确认后的推送

2026-10-24用户明确授权按既有fork快进边界推送及核对结果。首轮 `2fcfe205→2fdbb031` 推送并读回一致；本记录随文档提交一并推送。最终本地/远端SHA与清洁工作树见证据 `formal/push-receipt.json`、远端/CI状态见 `formal/remote-readback.json`。只读核对不视作CI通过，不额外dispatch或重跑成功测试，不强推/推origin/创建或合并PR。
