# 本地恢复和环境准备

## 一、只读校验后在新目录恢复
在解压包根目录：

```powershell
python .\verify_package.py
Get-Content .\STATE.json
```

`STATE.json.source_head`是权威交付提交。选择一个尚不存在的目录恢复（不要直接复制下面的目录名覆盖已有文件夹）：

```powershell
git clone .\history\metteur.bundle <新的恢复目录>
git -C <新的恢复目录> rev-parse HEAD
git -C <新的恢复目录> status --short
```

对照STATE，要求提交一致、工作区干净。`source/`为完整可读副本；优先用bundle保留历史。原本本地工作区已配置，不另行强制规定磁盘路径；先检查已有目录与改动，再决定如何整合。

克隆后的origin可能是bundle文件，不是GitHub。先 `git remote -v`，只读核对，暂不变更推送配置。若本地有更新，使用共同祖先和diff审阅，不能以交接包覆盖全部本地改动。

## 二、工具链
- Rust1.99.0，遵循rust-toolchain.toml和Cargo.lock。
- Node24.13.1（或经明确验证的兼容24版本），pnpm11.24.0，使用modules/webcore/package.json的packageManager与pnpm-lock.yaml。
- Python3.11/3.12。
- Windows x64 MSVC开发终端、Windows SDK、CMake、protoc、LLVM libclang；`LIBCLANG_PATH`指向实际libclang.dll目录。`where.exe link`应是MSVC linker，避免Git GNU link。
- WSL可供辅助开发，但不能代替原生Windows证据。
- 如缺依赖，列出官方来源及安装项；凭据/网络安全配置按本地授权操作，不以交接包自动授权任意安装。

## 三、最小构建（先核实际环境）
从恢复仓库根目录：

```powershell
cargo build --locked -p metteur-daemon --bin metteurd -p metteur-web-server --bin metteur-web
Set-Location modules/webcore
corepack pnpm install --frozen-lockfile
corepack pnpm exec vue-tsc --noEmit -p tsconfig.app.json --ignoreDeprecations 6.0
corepack pnpm build
```

当前有效typecheck预期暴露既有错误；先按修复清单处理，不说这是环境偶发。构建若使用CARGO_TARGET_DIR，后续启动必须指向同一目录。安装依赖可能访问官方包注册表，但不应调用模型API。

## 四、本地无付费演示入口
依赖及debug二进制、前端dist准备好后，从仓库根目录运行：

```powershell
node modules/webcore/scripts/run-nx-local.mjs --serve-only
```

它创建独立测试工作区、loopback可控provider和服务，不加载用户实际模型配置。读取控制台/ready.json中实际URL与workspace，不硬编码云端18981端口。

可选环境变量：METTEUR_VALIDATION_DIR、CARGO_TARGET_DIR、METTEUR_DAEMON_BIN、METTEUR_WEB_BIN、METTEUR_WEB_DIST、METTEUR_WEB_PORT。值必须按本机选择，不能复制云端 `/workspace/scratch/...` 路径。

`M3_LOCAL_BLUEPRINT`是夹具识别词，仅在这个隔离provider的新会话中使用，触发一个需人工确认的本地蓝图提案；Decline应结束。不要把它当成真实模型成功。Job样例的`sleep 8`需要本地先核shell适配。

## 五、原生Windows必要验证
从干净候选仓库根目录，以一个尚不存在的绝对目录：

```powershell
.\scripts\nx-windows-targeted.ps1 -OutputDirectory <绝对的新证据目录>
```

只执行锁、storage、恢复三组，失败即保留结果并修复；不要开启全量矩阵。脚本不自动安装依赖、不调用模型。PowerShell执行策略若阻止脚本，按当前系统/用户策略处理，不擅自全局放宽安全设置。

## 六、可选离线页面检查器
参考 `docs/nx-m6-completion/RENDERER.md`，以宿主受信任配置设置绝对路径：
- METTEUR_PREVIEW_NODE：实际node.exe。
- METTEUR_PREVIEW_RENDERER：本仓库modules/webcore/scripts/preview-inspection/render.mjs。
- METTEUR_PREVIEW_CHROMIUM：已安装Chromium路径；否则需要已安装的Playwright Chromium。

不禁用沙箱，不使用用户浏览器profile，不让插件选择任意命令/URL。缺配置显示not_configured；失败显示failed，不改成假成功。包内样例签名使用公开测试密钥，只适合隔离演示，不是生产信任凭据。

## 七、数据与回退
停止写入后完整备份数据库和工作区，或使用项目checkpoint机制；不要随意复制RocksDB部分文件。保存Prompt源、个人资源、日志、journal、报告与批注。旧二进制不能保证读写新schema；先在副本验证，禁止手改schema标记“兼容”。
