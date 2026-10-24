# IDE 修复真实模型记录

2026-10-24；独立授权 US$1、最多40请求。证据根 `F:/space/06_Projects/Agent/ide-usability-evidence/20261024`。此前179次/US$0.459912为历史记录，未导入本轮账本。

## 结果与费用口径

真实链路为生产 Web→grpc-web→Windows daemon→受控 relay→CommandCode 的 `deepseek/deepseek-v4.1-flash`。冻结任务在 `model-real-first/frozen-specs.json`，复验文件完全相同，SHA256 `7d27e39c553fc3dd75382b1156b55665240bbd1f9d40ee3c059c5783670bace5`。无自动重试；每次申请先写独立持久预算账本，未知回执阻止后续付费请求。真实提供方密钥只保留在 Python 进程，宿主使用本地 relay 假密钥，回执不打印真实凭据。

`model-budget.json` 共 **30次已结算请求**，计价上界 **US$0.0935517**，账本内 reserved/unknown 均为0；包含失败首轮12次及定向复验18次。使用官方 [Flash 定价](https://commandcode.ai/models/deepseek-v4-1-flash) 的峰时输入US$0.30/M、输出US$1.20/M保守计费，不用离峰或缓存折扣，不代表已核对提供方发票。

另有一次本地故障脚本用未配置模型名触发 provider HTTP401，未经过 relay，单列 `provider-auth-rejection.json`。宿主记录1次请求、0 tokens/费用，但 completeness=false，提供方未返回usage；不能把它写成已结算或已核实零账单。该路径立即停止，故障验收改用配置内本地503。**外部尝试合计31次**，仍小于40；之后不再付费调用。费用上界数字仅对应30次已结算请求，401的提供方账单未另核实；余额不自动用于未来阶段。

## 三个冻结目标

| 目标 | 客观验收 | 定向复验 |
|---|---|---|
| catalog_read | 模型实际查目录、审阅草稿、审批保存；Start常量→ReadFile.path为数据边；独立执行实际读出中文标记，文件不变 | 通过 |
| catalog_llm_branch | 实际生成CallLLM、Contains、Branch和两个End；保存后独立执行真实ReadFile、持久正文与工具结果，实际遍历True边 | 通过 |
| manual_requirement | 模板编译的人工要求出现在真实提供方请求；真实节点返回中文证据，确定性命令实际执行，原文件不变 | 通过 |

报告为 `model-real-catalog-retest/summary.json`，各目录 report.json 保存宿主事件、审批、蓝图身份和实际 Output；`request-receipts.json` 记录人工要求是否进入真实payload，两个模板节点请求均为true。模型说“成功”不作为判定。模型不自行执行它生成的蓝图，保存审批与操作员的独立运行分开；拒绝越出冻结节点/路径/预算范围的审批。

`model-ui-readback.json` 和 `model-ui-offline.json` 另证实：重启宿主后正式浏览器能读取真实模型结果；断连保留已记录正文并明确显示，恢复/刷新仍可读，零Send/Run。不是以受控provider冒充模型理解能力。

## 首轮失败与解释

首轮模型两次将 Start 常量包进 data，真实编译器分别拒绝不存在的 filePath/data.filePath 输出；之后自行修正为有效连线并生成可审阅草稿。验收辅助脚本又误将编译器正常产生的 Tool(tool_name=ReadFile) 当作未知节点拒绝；报告写出后驱动仍未退出，保留了所有12次请求，最终只停止该自有驱动，未自动重跑。

后续补清目录中的 Start 直接参数示例和 Tool 别名编译说明，修正辅助脚本的别名校验及结束处理，再执行同一冻结样本；三项通过。首轮 report/log/账本未删除或合并成“首轮全通过”。401及本地503故障测试回执也分别保留，后者只验证真实服务错误呈现，不计入真实模型能力证据。

这三个合成任务证明上述目录、编译、保存与执行链路可工作，不能证明所有节点、长任务、复杂函数/循环都可靠，也不支持模式优劣结论。旧三模式矩阵、首轮失败、人工接受项保持原记录，不为新结论重复成功测试。
