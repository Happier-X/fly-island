# NSH 发版与更新链路（2026-11-25）

## 打包（2026-11-27，NSH 0.2.0 已实跑）

本机已安装 Inno Setup 6.7.3，不需要再次安装。第一版不签名，邮箱配置延期。
使用 `nsh/scripts/package.ps1`，不要将 `nsh package` 的默认 runner 当成发布程序。
完整入口是 `nsh/host/src/main.rs`：宿主负责 Win32 消息循环、托盘、全局快捷键、
DPI、拖放与截图联动。只使用 `focusd-island-host.exe`，不混入第二个应用 exe。

前置条件由主人预先准备：PowerShell 7、仓库固定的 Rust 工具链、x64 MSVC target、
MSVC/Windows SDK、NSH CLI 0.7.2 及 Cargo.lock 对应的本地依赖缓存。
脚本不安装依赖；Cargo 使用 `--locked --offline`，通过不带 `--install` 的
`rustup run` 使用已有工具链。缺少依赖/工具链时停止，由主人决定后续处理。
NSH CLI 必须提前可用；脚本不运行其安装或 package 命令。

在独立副本根目录执行（以下命令本轮均未执行）：

```powershell
# 完整宿主 + NSH 资源 + 便携 zip + 未签名安装器
pwsh -NoProfile -File .\nsh\scripts\package.ps1
# 复用本脚本此前在此副本生成的宿主和资源；不构建源码
pwsh -NoProfile -File .\nsh\scripts\package.ps1 -SkipBuild
# 只要便携目录/zip，不调用 ISCC
pwsh -NoProfile -File .\nsh\scripts\package.ps1 -PortableOnly
# ISCC 安装在其他位置时显式指定（不重新安装）
pwsh -NoProfile -File .\nsh\scripts\package.ps1 -IsccPath 'D:\Tools\Inno Setup 6\ISCC.exe'
# 老 Tauri 用户在线升级用的 NSIS 桥接包（输入是上面产出的 portable 目录）
pwsh -NoProfile -File .\nsh\scripts\tauri-bridge.ps1 -PackageDir nsh\.package\dist-0.2.0\portable -BridgeVersion 3.6.1
```

输入与产物均在副本内；不依赖 `C:/code/fly-island` 下的编译产物：

- `nsh/.package/target/x86_64-pc-windows-msvc/release/focusd-island-host.exe`：显式构建 `nsh/host/Cargo.toml` 的宿主。
- `nsh/.package/resources/`：由现有 `nsh build` 生成的 `app.json` 和 `assets/`，不手写运行时格式。
- `nsh/.package/dist-<version>/portable/`：宿主 exe、`nsh/app.json`、`nsh/assets/`、许可证、NOTICE、AirSnip 依赖声明和 `UNSIGNED.txt`。
- 同级 `FocuSD-Island-NSH-<version>-win-x64.zip`：便携更新包，根目录只有一个宿主 exe，兼容更新器的包根/程序发现逻辑。
- 同级 `FocuSD-Island-NSH-<version>-win-x64-setup.exe`：仅在 ISCC 成功后才存在。
- 同级 `SHA256SUMS.txt`、`package-inputs.json`：实际产物哈希、宿主/资源清单哈希与是否跳过构建；不是签名或源码一致性证明。

版本读取 `nsh/nsh.json`，必须与 app/host 两份 Cargo.toml 一致，目前为 `0.2.0`。
版本号同时决定暂存目录 `nsh/.package/dist-<version>`：暂存刻意留在 `.package` 下，Inno Setup 的 260 字符路径上限容不下带完整包名的深路径。同版本输出目录已存在时拒绝覆盖；请先人工保留或移走，失败残留也不自动删除。
`-SkipBuild` 不保证二进制与当前源码一致，发布者必须自行确认输入来源。
宿主翻译及提示音已由源码内嵌；随包仍携带完整 NSH 生成资源，不重新实现资源生成。
构建的 Cargo target 和 TEMP/TMP 定向到 `nsh/.package/`；不改变全局环境配置。

### 安装行为

配置在 `nsh/installer/focusd-island.iss`。使用独立 `com.focusd.island.nsh` 标识，
默认安装到当前用户的 `%LOCALAPPDATA%\Programs\FocuSD Island NSH`，不请求管理员权限。
安装向导保留目录选择和开始菜单文件夹选择；默认创建开始菜单应用/卸载入口，
桌面快捷方式默认不勾选；所有应用快捷方式的工作目录为安装目录。
自选目录请保持当前用户可写，否则现有在线更新器无法覆盖文件。

勾选“开机自动启动”时写当前用户 Run 值 `FocuSD Island (NSH)`，与宿主托盘菜单里的同一开关共用；
卸载时由 Inno 的 `uninsdeletevalue` 移除。托盘图标读取安装目录下的 `app.ico`，缺失时回退系统默认图标。
系统应用列表中注册卸载项；卸载只按 Inno 日志移除其安装文件和快捷方式，
不递归删除用户数据/旧 Tauri 数据；便携更新新增的文件可能需主人手工清理。
安装前从托盘退出 NSH；安装器不自动关闭/重启宿主，安装结束也不自动运行应用。
- 安装器界面使用 Inno 自带英文语言，未签名提示用自定义安装前说明替换；不下载额外语言包。
`UNSIGNED.txt` 在安装前展示并随包安装；不修改账号，不配置邮箱。
安装器图标复用仓库已有 `src-tauri/icons/icon.ico`，不引入新的图标生成过程。

### 主人构建与安装清单（本轮未执行）

- 按上述命令构建；确认日志构建的是 `nsh/host/Cargo.toml`，资源来自固定版本 NSH CLI；检查依赖/运行库在干净机器上是否齐全。
- 检查便携目录与 zip 的 exe 名和资源布局，核对 SHA-256；从任意工作目录运行宿主，检查托盘、全局快捷键、DPI、截图和资源显示。
- 在当前用户可写且含中文/空格的自选目录安装；确认目录页、开始菜单页、桌面选项默认不勾选及系统卸载项。
- 分别检查开始菜单和可选桌面快捷方式；检查 SmartScreen/未签名说明，不能声称签名受信任。
- 关闭 NSH 后覆盖安装后续版本；核对独立更新 zip 的下载/重启链路，避免选中安装器 exe。
- 卸载后检查快捷方式及系统卸载项移除、NSH 数据保留、Tauri 安装/数据不变；邮箱及其余需人工授权的项目留给主人。
- 按 GPL-3.0 提供对应的完整源码及构建说明；分发前复核第三方资源与依赖的许可要求。

本轮没有宿主二进制，未生成安装器；只交付配置和构建命令。
未运行脚本、NSH CLI、构建、测试、安装/卸载或 GUI；资源生成命令与 ISCC 配置未实跑，
不能称安装、资源加载、干净机依赖或更新链路已验证。

便携版不注册卸载项；应用运行后的数据在 `%APPDATA%\com.focusd.island.nsh`。

## 更新清单

端点在设置 → 自动更新中填写，留空关闭。清单是独立 JSON：

    {
      "version": "0.1.2",
      "url": "https://example.com/FocuSD%20Island%20(NSH).zip",
      "sha256": "64 位小写十六进制",
      "signature": null,
      "notes": "本次更新说明"
    }

- version 必须是严格 semver，且高于当前版本才会提示。
- url 指向便携 zip；生产环境应使用 HTTPS。本地测试允许 HTTP。
- sha256 必填，下载后强制校验；缺失或格式错误直接拒绝。
- signature 可选，只作为元数据展示；当前没有公钥配置和签名校验，不能把有该字段当成“已验证”。
- notes 可选，显示在更新设置页。

宿主启动时如果端点非空，会在后台检查；设置页也提供“检查更新”。发现新版本后显示版本、notes 和签名状态，点击“下载并重启”才执行下载。

## 下载与替换

1. 下载到 %TEMP%\focusd-island-nsh-update，限制 240 MB。
2. 计算 SHA-256，与清单严格比较。
3. 解压 zip，找到包含 nsh/app.json 的包根。
4. 覆盖目标目录：默认是当前 exe 目录；可用 NSH_UPDATE_TARGET_DIR 指向临时目录做验收。
5. 当目标是当前安装目录时，写一个 cmd 辅助脚本，等待当前进程退出后 xcopy 覆盖并启动新 exe；临时 target 会直接复制并启动新 exe。

失败状态：HTTP 404/网络错误、SHA-256 不符、zip 无效、找不到 app.json、目标不可写、启动失败，都会回到设置页的错误文本，不删除已有安装。

## 发布与安全

- 当前没有代码签名证书，便携版/zip/安装器均为未签名。Windows SmartScreen 可能显示“Windows 已保护你的电脑”，需要用户确认；这不是已验证签名。
- 强制 SHA-256 只能防止下载损坏/被替换后的内容不一致，不能证明发布者身份；谁控制端点谁就能提供新包。
- 生产应使用 HTTPS、限制清单/包访问权限，后续有证书时再补 Authenticode 和清单签名；在此之前文档和 UI 不得显示“已签名”。
- 回滚：保留上一版便携目录/zip；NSH 更新不修改 Tauri 数据，也不自动删除旧数据。

## Tauri 版本过渡

- 两版并存：Tauri 使用 %APPDATA%\com.focusd.island 和 com.focusd.island；NSH 使用 %APPDATA%\com.focusd.island.nsh 和 com.focusd.island.nsh。localStorage、LevelDB、凭据条目和更新端点互不覆盖。
- NSH 首次启动只读导入旧设置/剪贴板等白名单字段；旧文件不修改、不删除。
- 旧 Tauri 版建议在 NSH 版设置、剪贴板、消息、外观均确认后再卸载；卸载前保留旧数据目录作为回滚。
- 若 NSH 更新失败，关闭 NSH 后重新运行上一版便携目录即可；不要用 Tauri 的安装器覆盖 NSH 目录。
- 老用户迁移走 Tauri 自带的更新器：`nsh/installer/tauri-update.nsi` 由 `tauri-bridge.ps1` 编译成
  `fly-island_<bridge>_x64-setup.exe`，用 `~/.tauri/fly-island.key` 签名后写进 `latest.json`
  的 `windows-x86_64.signature`。更新器只会以 `/passive /UPDATE` 或 `/S /UPDATE` 调它，
  桥接包两种参数都按静默安装处理。
- 桥接包会结束旧进程、删除 `%LOCALAPPDATA%\fly-island`、旧 Run 值和旧卸载项，再装 NSH 版并拉起岛；
  旧数据目录 `%APPDATA%\com.focusd.island` 保留。
- 开机自启只在旧版原本开着时才给新版也开：桥接包先读旧 Run 值再删除，不替主人改设置；
  同时清掉早期 Inno 版留下的 `unins000.*` 和“FocuSD Island (NSH)”快捷方式分组，避免出现两套入口。
- 桥接版本号必须高于旧 Tauri 版的当前版本（本轮 `3.6.0` → `3.6.1`），并让该 release 成为 Latest，
  `releases/latest/download/latest.json` 才会返回新清单。
- NSH 自身的更新清单（`app.json` + zip + SHA-256）与 Tauri 的 `latest.json` 是两套格式，不要互相顶替。
- 已发布资产（`v3.6.0-nsh` 的 Inno 包、zip）保持不动；桥接包是独立文件，不覆盖旧资产。
- 第一版 NSH 必须包含截图套件；不能以“截图继续留在 Tauri 旧版”作为发布口径。截图由独立专项任务实现，但其完成是首版发布前置条件。
- 截图现状（2026-11-25）：S1–S3 完成，混合 DPI/HDR 通过；Chromium 长截图与旧版一致受限（旧版 600/855 仅首屏，NSH 600 可连续拼接、855 同旧版），按“与现版持平”记录，不算回归；NSH 设置页已补齐自定义色板、命名模板、工具键、工具栏显隐与界面缩放，对丢矩阵无未分类行，截图不再阻塞发布。详见 docs/architecture/nsh-screenshot.md。

## 迁移验收（2026-11-27 实测）

- 真实路径：本机 Tauri 3.6.0 的自带更新器检查到 3.6.1，下载后以 `/passive /UPDATE` 调桥接包（临时目录名 `fly-island-3.6.1-updater-*`），静默安装完成并自动拉起岛。
- 安装后目录只有宿主、`app.ico`、`nsh/`、许可证、`uninstall.exe`；宿主 SHA-256 与打包产物一致；窗口 320×40、可见、主屏顶部居中。
- `%LOCALAPPDATA%\fly-island`、旧 Run 值、旧卸载键、旧快捷方式全部清掉；系统应用列表只剩 `FocuSD Island (NSH) 0.2.0`。
- 手动以 `/passive /UPDATE` 重复运行验证幂等与自启保留：旧自启不存在时不新开；模拟旧自启存在时迁移为 `FocuSD Island (NSH)` 并删除旧值。
- 未测：卸载全流程、开机自启在真实登录时生效、托盘图标的视觉确认。

## 历史验收（2026-11-25，非本次安装器验证）

- 本地 Python HTTP 服务 127.0.0.1:8765 提供 manifest 与 zip；宿主启动后自动检查到 0.1.2。
- 点击“下载并重启”：下载、SHA-256、解包、复制到临时 target、启动新 exe 全流程通过；target 出现新 exe 与 nsh/app.json，新进程运行。
- 截图：docs/assets/nsh-release/2026-11-25/update-settings-125.png。
- 自动化：cargo test --lib --offline 包含清单版本/哈希、HTTP 404、哈希不符、目标不可写测试；全部通过。
