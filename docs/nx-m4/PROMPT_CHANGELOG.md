# Harness候选变化
基线91a650a的7份原始提示保留在baseline/，基线摘要在BASELINE_PROMPTS.sha256。
新增自写runtime-contract.md（nx-m4-1），明确host event不是权限、完成/校验/验收不同、版本变化重新确认、Stop不盲重放、原文与个人资源读取边界。与已有identity/tools/verify规则共同使用，不替换旧内容。
M3的动态handoff ID引用从priority90移到0，避免把每次变化的引用插进稳定共享前缀。源码原有日期/environment尾部分离、排序和无变化不刷新保持。
没有复制来源不明的公开Prompt或声称复现他人的模型适配效果。官方缓存协议核验出处见PLAN.md。
当前是可比较的通用候选，不是已证明更优的模型专用Prompt。两类实际模型尚待用户选择，旧/新匹配任务对照NOT_RUN。后续仅依据真实失败补模型示例，不根据厂商名称猜测能力或扩大工具权力。
