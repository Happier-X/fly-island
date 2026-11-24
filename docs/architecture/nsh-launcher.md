# 快捷启动器 parity（NSH）

状态：2026-11-24 主体完成；↑/↓ 键盘选择已接入低级键盘钩子，待人工按键复核。Tauri 行为以 `src-tauri/src/quick_launcher/` 与 `src/features/quick-launcher/` 为准，NSH 实现位于 `nsh/src/launcher/` 与 `nsh/src/launcher_ui.rs`。

## 对照表

| 能力 | Tauri 现版 | NSH 实现 | 结论 |
|---|---|---|---|
| 唤出入口 | 全局快捷键 Alt+Space，或双击 Ctrl；设置可改 | `RegisterHotKey` 组合键；`WH_KEYBOARD_LL` 识别两次干净 Ctrl 点按 | 对齐，两种触发都可用 |
| 窗口形态 | 680×460，无边框、透明、置顶、跳过任务栏；鼠标所在显示器居中；Esc / 失焦关闭 | NSH `Normal` 窗口，680×460，`transparent`、`AboveDock`、`frame none`、`taskbar false`、`close_on_blur`；`display_under_cursor` 选屏 | 对齐；副屏 150% 实测 1020×690 |
| 应用索引 | 开始菜单 `.lnk/.url/.appref-ms` 递归 4 层；App Paths 注册表；跳过 uninstall/readme 等 | 开始菜单同样递归 4 层；`Get-StartApps` 的 UWP 以 `shell:AppsFolder\<AppID\>` 打开；PATH 下 `.exe`；同样的跳过词；按目标去重 | 覆盖更广（含 UWP/PATH）；上限 4000 |
| 文件搜索 | 默认 Desktop/Documents/Downloads；自定义目录；递归 3 层；上限 3000；跳过隐藏/系统/重解析点与 node_modules/target/.git 等；30 秒缓存 | 同左 | 对齐 |
| URL | 输入即网址直接打开；否则按 Bing/Google/Baidu/DuckDuckGo 搜索 | 同左，`reqwest::Url` 编码查询词 | 对齐 |
| 排序与分组 | exact > prefix > word > contains > path；应用优先于文件；短名优先；同目标去重；最多 30 条 | 同左 | 对齐 |
| 键盘 | ↑/↓ 循环选择，Enter 打开，Esc 关闭；输入法组合期间 Enter 只提交组合 | 低级键盘钩子观察 ↑/↓/Enter/Esc，UI 每 50ms 轮询执行 | Enter/Esc 已实测；↑/↓ 待人工按键确认（合成输入未驱动 NSH 窗口级绑定） |
| 鼠标 | 悬停选中，单击打开 | 同左 | 对齐 |
| 空/加载/无结果 | 空查询提示；索引中提示；无结果提示；打开失败显示错误 | 同左，底部状态位显示“正在建立索引…”或错误 | 对齐 |
| 索引刷新 | 启动时后台建应用索引；应用 300 秒、文件 30 秒过期；每次打开检查 | 同左；设置里的目录变化会换 key，下次打开重建 | 对齐 |
| 打开方式 | Tauri opener 插件启动应用/文件/URL | `ShellExecuteW`；只允许绝对路径、http(s)、`shell:AppsFolder\`；失败写到底部 | 对齐 |
| 图标 | Shell 图标转 PNG，按结果异步取图 | 目前用应用/文件/网页三类字形 | 未对齐（不影响搜索与打开） |
| 设置 | 启用、触发方式、录制快捷键、冲突提示、重置、搜索项、引擎、目录 | 同左；旧 LevelDB 的 `focusd-island-launcher-settings` 一次性导入，不涉及密钥 | 对齐 |

## 实现位置

- `nsh/src/launcher/mod.rs`：索引缓存、TTL、后台重建、去重、`ShellExecuteW`、鼠标所在显示器。
- `nsh/src/launcher/apps.rs`：开始菜单快捷方式、UWP、PATH 可执行文件。
- `nsh/src/launcher/files.rs`：目录递归、跳过规则、30 秒缓存。
- `nsh/src/launcher/matching.rs`：匹配等级、应用优先、URL 直达、搜索引擎回退。
- `nsh/src/launcher/hotkey.rs`：组合键、双击 Ctrl 与窗口内 ↑/↓/Enter/Esc 观察，独立 Win32 消息线程。
- `nsh/src/launcher_ui.rs`：窗口、结果列表、空/加载/错误状态、设置分组。
- `nsh/src/preferences.rs`：`launcher_*` 设置与旧 LevelDB 导入。

## 验收（2026-11-24）

截图目录：`docs/assets/nsh-launcher/2026-11-24/`。

- 空状态：`empty.png`，680×460 居中打开，提示“输入应用名、文件目录里的文件名，或网址”。
- 应用：`app-code-results.png` 输入 `code` 得到 codex、code-tunnel、Visual Studio Code 等；`app-result.png` 输入完整名称后唯一结果。点击 Visual Studio Code 结果后打开 `package.json - Visual Studio Code` 窗口。
- 文件：`file-result.png`，把 `%TEMP%\nsh-launcher-files` 设为自定义目录后输入 `launcher-test-unique`，结果为目标 `.txt`。
- URL：`url-result.png`，输入 `https://example.com` 显示“打开网址 / 网页”，Enter/单击后交给默认浏览器。
- 副屏与 DPI：`secondary-150.png`，在 150% 的副屏上窗口物理尺寸 1020×690，位置 (3170,360)，位于该屏工作区居中。
- 键盘：`keyboard-app-result.png`/`keyboard-app-open.png` 为键入 `Notepad`、提交组合后 Enter 打开记事本；`keyboard-file-result.png`/`keyboard-file-open.png` 为键入 `launcher-test-unique` 后打开 Documents 里的文件；`keyboard-url-result.png`/`keyboard-url-open.png` 为键入 `https://example.com` 后 Edge 打开该页；`keyboard-cancel.png` 为 Esc 关闭后的桌面。Enter 与 Esc 在切换钩子前已实机验证；钩子版的 Enter/Esc/↑↓ 需人工按键复核。
- 索引重建去重：`cargo test --lib -- --ignored launcher::tests::the_real_app_index_has_unique_targets` 实测 1125 个应用，连续两次枚举目标唯一且数量一致。
- 构建：`cargo check --lib`、`cargo test --lib`（77 passed / 10 ignored）、`cargo check --manifest-path nsh/host/Cargo.toml` 通过；宿主完整 build 在键盘钩子改动前通过。

## 已知差异与后续

- 应用图标仍是字形，不取 Shell 图标。
- `.lnk` 的副标题显示快捷方式路径；Tauri 显示解析后的目标路径，打开行为不受影响。
- UWP 列表通过 `Get-StartApps` 获取，启动时有一次 PowerShell 子进程开销。
- 双击 Ctrl 与 ↑/↓/Enter/Esc 共用低级键盘钩子；只观察、不吞键，窗口失焦后仍由系统处理。
- ↑/↓ 的钩子轮询在合成输入（SendKeys/SendInput）下未观察到选中变化，需用真实键盘复核；Enter/Esc 已实测。
- 设置中的目录仍为手动路径输入（与 Tauri 相同），没有文件夹选择器。
