# 当前候选真实模型验收方案（待新授权）

2026-10-23。当前额度仍为0；以下是可审阅方案，不是调用授权。历史US$5及旧DeepSeek结果不复用。仅在用户明确批准后运行，候选须对应届时冻结源码/二进制/Web。

建议沿用现有CommandCode适配：HTTPS `https://api.commandcode.ai/provider/v1`，模型 `deepseek/deepseek-v4.1-flash`，仅flash一种模型、当前候选一次。不做付费baseline对照或无理由重跑。凭据只在本机读取现有配置文件，不写入实例配置、普通日志或对话；daemon只取得本地中继地址和临时假key。若配置不可用，再获取无密钥的配置路径，不索取明文key。

| 合成任务 | 客观退出条件 | 允许数据与动作 |
|---|---|---|
| dev-03 跨文件缺陷修复 | 修复shared.normalize；两个调用方和签名保持；实际unittest通过，报告真实结果 | 仅shared.py、alpha.py、beta.py、test_shared.py；修改shared.py；执行`py -3.12 -m unittest test_shared.py` |
| hel-10 蓝图交接与只保留结果 | 具体版本确认后仅一个run；结果保留，零自动续聊dispatch | 新建合成evidence.txt及现有审阅蓝图；原RequestBlueprintRun、审批和结果接口 |
| hel-12 Job断连重接 | 一次受控延时Job；同一源/会话恢复，最多一个dispatch，不另起Job | 新建合成evidence.txt；仅既有`node -e "setTimeout(() => {}, 8000)"`及AwaitEvents |

三项seed及摘要已冻结在证据根`s4-real-model-plan/seeds`；不会把旧阶段输出改名为当前结果。真实文件/测试/进程产物由宿主和外部检查判定，模型自述不代替验收。每项记录完成、人工介入、意外修改、耗时、实际用量及未知值。

申请本阶段**新增独立上限US$1、全组最多36次模型请求**（包括parent/child/自动续聊），不自动扩额。继续复用既有durable预算中继：逐请求按正文UTF-8字节加8192协议余量及输出上限保守预留，未知费用阻断后续调用，无自动重试/重定向。新账本历史导入为0；调用次数上限重开后仍有效，不能静默提高。离线守卫已检查，无付费调用。

估算采用峰时上界输入US$0.30/M、输出US$1.20/M，不依赖缓存折扣；假设全组30万输入及3万输出tokens，约US$0.126。实际费用以provider报告和账本为准，未知不记0。当前价格已按[CommandCode官方模型页](https://commandcode.ai/models/deepseek-v4-1-flash)核实；峰谷价格有别，本方案统一用峰时预留。

运行入口复用`scripts/nx_model_eval.py`：`--phase all --cases dev-03 hel-10 hel-12 --models flash --candidate-only --fresh-budget --limit-usd 1 --max-model-requests 36`，另传本机credential-config、全新budget-ledger/output以及冻结target/web-dist/node路径。未批准前不执行该调用命令。不得覆盖旧输出或旧账本。
