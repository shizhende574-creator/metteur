# 固定样本性能记录

2026-10-23，Windows当前机器。候选b6a451d的冻结程序/源码摘要在`s4-runtime-b6a451d/manifest.json`；Web为`s4-production-first`。对照使用保留的190b48d生产运行版19088，候选19091。原DW-00 dev/mock记录模式不同，不作为生产速度提升证据。

同一Chromium进程、1360×900、新浏览器context、相同两文件内容；先对照后候选，每版20次热标签切换、30次插入并等待文字可见。阈值沿用BASELINE.md预先约定的max(绝对预算, 对照×1.25)。原始样本、机器/请求/时间/截图见`s4-performance-first/measurements.json`，首轮通过。

| 测量 | 对照 | 候选 | 阈值 |
|---|---:|---:|---:|
| 浏览器新context至文件树可见 | 458ms | 225ms | 5000ms |
| 热切页p95 | 93ms | 79ms | 500ms |
| 插入至可见p95 | 37ms | 40ms | 100ms |
| 编辑后JS used heap快照 | 32.1MB | 33.5MB | 观察值，无先验通过阈值 |

宿主程序预先运行，启动项不是冷daemon启动。单轮顺序测量不能证明统计性提速。浏览器自动化的插入不是实际Windows IME，后者仍需用户确认。候选与对照均无pageerror。

每版Tasks空闲10秒均无QueryContinuations/GetConciergeState/GetOversightReviews的1s/1.5s轮询。可见请求是最后一次延迟恢复备份，以及既有5秒ListWorkspaces基础状态；不把基础连接/工作区读取或本地时钟隐去。另有S3定向重复通知/Usage/Reviews合并证据，未因性能测量重跑该场景。

持续观察将单列实际开始/结束时间、样本次数、恢复记录数量和内存，不将上述短区间称为长期稳定性通过。1000次同稿备份的真实DB恒定行数/无业务通知属于独立存储检查，不等于磁盘物理字节不增长。
