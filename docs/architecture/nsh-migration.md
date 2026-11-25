# FocuSD Island → NSH 迁移评估（Phase 1）

> 状态：Phase 1 可行性 spike 与评估。现有 Tauri 实现不改动，`nsh/` 与新文档独立提交。
> 验证环境：Windows 11 26H2（build 26300）、NSH 0.6.1（2f51d86）、Rust 1.99.1。

## 结论

**可行，但不能整体平移。** NSH 已经能支撑岛屿的外壳与大部分业务界面：无边框、透明、半透明、`aboveDock` 置顶、动态改宽高、托盘、全局快捷键、i18n、剪贴板读取都在 `nsh/` spike 中跑通，Rust root 不需要 QuickJS。

主要受限点是 **Windows 原生能力**：`nsh run` 的 Rust root 不暴露自己的 `HWND`，也没有低级键盘钩子、剪贴板变更通知、系统媒体会话、UIA/窗口读取、屏幕捕获与 OCR、凭据管理器。Phase 2 已用“自有 Win32 消息循环 + `AppView` 顶层窗口”拿到 HWND；其余能力仍需继续用 `windows` crate 自写。截图套件（HDR、长截图、贴图、混合 DPI）风险最高，已改为单独立项。

推荐路线：**Rust root 的纯 NSH 应用**，把现有 `src-tauri` 的领域模块逐步改成可复用的 Cargo crate；需要 `HWND` 的能力先用独立 Win32 辅助窗口实现，只有在“逐像素点击穿透 / AppBar / DWM 自绘 / 捕获叠加层”成为硬需求时，才升级为 `AppView` 混合宿主。

> **2026-11-24 Rust-only 更新：** NSH 版不再有 TSX/QuickJS 应用入口。`nsh/App.test.tsx`、`nsh/tsconfig.json`、`nsh/tests/mock-openai.mjs` 已删除；`src/lib.rs` 的 Rust `App` 是唯一根，翻译 mock 改为 Rust 本地 TCP 服务。验证：`cargo test --manifest-path nsh/Cargo.toml --lib`（103 passed / 9 ignored）；`nsh build nsh` 输出 `no bundle: its runners mount the root`，产物只有 `app.json` 与 assets；`nsh run nsh` 输出 `running FocuSD Island (NSH) (Rust, no JS runtime)`；打包目录/zip 无 `.js/.tsx`/bytecode。下文 2026-11-24 之前记录里的 `nsh test` 是已删除的 TSX 测试，仅作历史。

## NSH 能力核对（0.6.1）

| 能力 | 结论 | 依据 / 限制 |
|---|---|---|
| `WindowKind` | 有 `normal` / `dialog` / `panel` / `popup` / `overlay` | `overlay` 透明、点击穿透、永不聚焦、`aboveDock`、所有工作区；不适合可交互岛屿。`popup` 失焦或 Escape 即关闭，不适合常驻主窗口。 |
| 主窗口形态 | 可行 | 主窗口用 `normal` + `frame: "none"` + `transparent: true` + `level: "aboveDock"` + `taskbar: false` + `activate: false`。spike 已验证。 |
| 置顶层级 | `level: "aboveDock"` | Windows 上为置顶层级，可压在任务栏之上；不是“像素级最前”，全屏游戏/独占程序仍可能覆盖它，需要自己判断全屏。 |
| 动态尺寸 | 可行 | `WindowRef::set_bounds(WindowFrame { x, y, width, height, animated })`；Windows 上尺寸变化是瞬时的，`animated` 只对 macOS 有意义。 |
| 半透明 | 整窗透明 | `transparent: true` 让未绘制区域透出桌面，半透明颜色正常混合；没有“按 view 做点击穿透”，也没有窗口级圆角阴影，需自绘。 |
| 位置 | 需自己算 | 没有“顶部居中”预设；用 `displays()[].visible_bounds` 计算并 `set_bounds`。spike 在首帧后按 `displays()` 居中，并随显示器变化重新居中；只在主显示器，未处理跨屏移动。 |
| 托盘 | 可用 | 每应用一个；`tray().icon(...).tooltip(...).menu(...).on_click(...)`；Windows 上 SVG 可能按模板色绘制，需实测彩色图标。 |
| 全局快捷键 | 可用 | `register_global_shortcut("mod+shift+i", ...)`，返回是否被系统接受；没有低级键盘钩子，不能识别“双击 Ctrl”这类序列。 |
| 开机自启 | 有，但契约不同 | `Application.setLaunchAtLogin` / `set_launch_at_login` 使用应用自己的登录项，不能保留现有注册表值 `FocuSD Island`。需要迁移方案或继续自写 HKCU Run。 |
| 多窗口 | 可用 | `window()` / `open_window`，共享信号与上下文；`dialog` / `panel` / `popup` 可覆盖设置、翻译、启动器。 |
| 嵌入宿主 | `AppView` | `nsh-platform-windows` 的 `AppView` 可放进自有 Win32 子窗口，并通过 `AppView::hwnd()` 拿到句柄；这是需要完全控制窗口时的退路。 |
| Rust root | 可用 | 不链接 QuickJS；首次构建约 60 个 crate（本机约 4m42s，增量 1.5s）。 |
| 翻译 | 可用 | `locales/` 的 rust-i18n 格式由 `nsh` 编译进 runner，`t!` / `set_locale` / `available_locales`；spike 已生成 `translations.json`。 |
| 剪贴板 | 读写可用，无事件 | `clipboard_string` / `clipboard_image` / `clipboard_files`；没有变更通知，只能轮询。 |
| 自动更新 | 有，但格式不同 | NSH 采用自研清单 version/url/sha256/signature/notes；SHA-256 强制，签名可选但未验证。与现有 Tauri 更新端点/格式不兼容，见 nsh-release.md。 |
| 打包 | 可用 | `nsh package --target windows` 输出目录、zip、Inno Setup 脚本；安装到 `%LOCALAPPDATA%\Programs\<Name>`，免管理员。 |
| 签名 | 自己做 | NSH 不签 Windows 可执行文件/安装包，需 `signtool`。 |

## Spike 范围与结果

代码在 `nsh/`，与 `src/`、`src-tauri/` 并存：

- 顶部居中的胶囊：折叠 320×40，展开 620×340，点击展开、点 ⌃ 收起；窗口尺寸同步变化。
- 真实剪贴板历史：每 800 ms 读一次系统剪贴板，去重、保留最近 20 条；点击条目写回剪贴板。
- 托盘：`assets/tray.svg`，左键切换岛屿，菜单含显示/退出。
- 全局快捷键：`Ctrl+Shift+I` 切换展开/收起并聚焦。
- 中英文：`locales/en.yml`、`locales/zh-CN.yml`，展开后 EN / 中文 按钮切换。
- 窗口：`normal` 类型 + `frame: none` + `transparent` + `aboveDock` + `taskbar: false` + `activate: false`。

运行：`nsh run nsh`（需要新终端让 PATH 生效）。`cargo run --manifest-path nsh/Cargo.toml` 也能启动，但拿不到 `nsh` 生成的翻译，界面会显示 key。

## 模块矩阵

| 模块 | NSH 下的做法 | 风险 |
|---|---|---|
| 剪贴板（文本 / 图片） | UI 用 NSH；文本用 `clipboard_string` 轮询；图片用 `clipboard_image`；收藏、备注、持久化用 Rust 文件存储。 | 中高：没有 `WM_CLIPBOARDUPDATE`，轮询有延迟/空转；图片每次比较和落盘成本高；文件剪贴板与拖出需 Win32/OLE。 |
| 翻译窗口 | NSH `panel`/`popup` 窗口；请求继续用 Rust `reqwest`，或 NSH `fetch`；快捷键仍用 `register_global_shortcut`。 | 中：凭据要留在 Windows 凭据管理器（NSH 没有）；输入法/焦点行为需验收。 |
| 媒体控制 | UI 用 NSH；SMTC 会话、封面、播放控制继续用 `windows` crate（WinRT），后台轮询。 | 中：NSH 只有音频播放，没有系统媒体会话；COM 线程模型要接好。 |
| 微信 / QQ / 邮件通知 | UI 用 NSH；UIA 读取、通知注册修复脚本、IMAP/附件继续用 Rust。托盘闪烁需要自己调 Win32。 | 中高：这些本来就是 Win32/UIA 密集模块；NSH 托盘不能直接复用“闪烁”语义。 |
| AI 状态（Cindy 通知） | Rust 只读 Cindy 数据库 + 本地水位；UI 用 NSH。 | 低：纯数据读取，NSH 只负责展示。 |
| 截图套件（框选、标注、长截图、OCR、贴图、HDR、混合 DPI） | 选区覆盖层用独立 Win32 窗口（或 NSH `overlay`，但需要交互，不能用点击穿透）；捕获用 DXGI / Windows.Graphics.Capture；OCR 用 Windows.Media.Ocr；长截图靠输入注入 + 滚动拼接；贴图是独立置顶窗口。 | 极高：HDR 色调映射、混合 DPI 坐标、全屏独占、输入注入、内存与性能都要单独验证；不应作为 Phase 2/3 的前置。 |
| 快捷启动器 | NSH `popup` 窗口 + Rust 索引/图标；图标转成图片资源或 data URL。 | 中高：双击 Ctrl 需要低级键盘钩子（自写 Win32）；大量文件扫描与图标解码的性能要重做。 |
| 设置窗口 | NSH `dialog`/`panel` 窗口，复用 NSH 基础控件；设置存储改到 Rust JSON 或 NSH storage。 | 中：设置项非常多，控件与窄窗口适配要逐项验收。 |
| 自动更新 | `nsh package` + 自研清单；Rust 应用校验 SHA-256、下载、替换、重启。 | 中高：签名密钥、manifest 格式、安装目录和旧 Tauri 版本升级路径都要重新设计。 |
| 外观主题 / 预设 | NSH 主题 token（`theme.json` / `setTheme`）+ 窗口 `backdrop`（Mica/Acrylic）；预设编辑器用 NSH 控件重写。 | 中：没有任意 CSS；现有主题变量、壁纸、布局编辑器要映射到 token 体系。 |
| i18n | NSH `locales/` + `t!` / `set_locale`；现有中英文字典可脚本转换成 rust-i18n 格式。 | 低：能力完整；注意运行时切换与系统语言回退。 |


## 必须自己写 Win32 或嵌入 AppView 的部分

NSH 的 Rust root 是原生代码，可以直接依赖 `windows` crate；问题不在“能不能调用 Win32”，而在 **拿不到 NSH 自己窗口的 `HWND`**，也没有对应的钩子/事件。独立应用里 `nsh_core` 没有公开 `hwnd()`；只有把 NSH 放进自有窗口（`AppView`）才有 `AppView::hwnd()`。

| 需求 | 现状 | 方案 |
|---|---|---|
| 逐像素点击穿透、异形窗口、AppBar、DWM 圆角/阴影、`SetWindowDisplayAffinity` | NSH 只有整窗 `pointerEvents` 和 `backdrop` | 自有 Win32 窗口 + `AppView`；或继续用独立 Win32 辅助窗口 |
| 全局低级键盘钩子（双击 Ctrl） | 只有 `register_global_shortcut` 单键 | Rust 自写 `SetWindowsHookEx(WH_KEYBOARD_LL)`；注意钩子线程与超时 |
| 剪贴板变更通知、图片/文件格式细节、拖出 OLE | 只有读写 API | Rust 自写隐藏消息窗口 + `AddClipboardFormatListener` / `OleSetClipboard` |
| 系统媒体会话（SMTC）、封面、播放控制 | NSH 只有播放音频 | 继续用 WinRT `GlobalSystemMediaTransportControlsSessionManager` |
| UIA / 窗口枚举 / QQ、微信通知读取 | 无 | 继续用现有 `win32/` 封装 |
| 屏幕捕获、HDR、OCR、输入注入、长截图、贴图 | 无 | 独立 Win32 捕获模块 + Windows.Graphics.Capture / DXGI / Windows.Media.Ocr |
| 凭据管理器 | 无 | 继续用 Windows Credential Manager |
| 现有 `FocuSD Island` 开机自启注册表值 | NSH 用自己的登录项 | 继续自写 HKCU Run，或做一次性迁移 |
| 全屏检测、托盘闪烁、任务栏/DWM 特殊行为 | 无 | 自写 Win32；NSH 的 `level` 只负责置顶层级 |

### 架构选项

| 方案 | 说明 | 优点 | 代价 |
|---|---|---|---|
| A. Rust root + 纯 NSH 窗口 | NSH 拥有岛屿、设置、翻译、启动器窗口；Win32 能力用独立辅助窗口/后台线程 | 最少胶水；窗口/托盘/快捷键/i18n 直接用 NSH；无 QuickJS | 主窗口 `HWND` 不可得；逐像素行为和捕获叠加层要另做窗口 |
| B. Rust root + TSX 页面（mixed） | Rust 提供领域模块，TSX 写界面；仍用 NSH 窗口 | 热更新快，适合大量设置页与实验 | 带 QuickJS；沙箱限制；Rust/TSX 边界要设计；启动与内存略高 |
| C. 自有 Win32 宿主 + `AppView` | 我们创建所有窗口，NSH 只渲染子视图 | 完全控制 `HWND`、层级、点击、捕获；可逐步替换 UI | 自写窗口循环/托盘/快捷键/多显示器；与 NSH 生命周期集成复杂；不能直接用 `nsh/desktop` 的大部分窗口能力 |

**最终采用 C′：自有 Win32 消息循环 + `AppView` 顶层窗口。** NSH 0.6.1 的 `AppView` 在子窗口模式下会关闭 `transparent`（`app.rs` 里 `options.transparent && parent.is_none()`），父子窗口无法保留像素级的逐像素透明，因此不再尝试“自有父窗口 + AppView 子窗口”。宿主仍完全控制 AppView 的 `HWND`、消息循环、托盘、快捷键、尺寸与焦点；截图、全局钩子、剪贴板监听仍做独立 Win32 模块。

## Phase 2–4 里程碑

### Phase 2：宿主、外壳与基础设施（已完成；双屏混合 DPI 已验收）

范围：

- 混合宿主：自有 Win32 消息循环、`AppView` 顶层窗口、HWND 控制、置顶/穿透/多屏 DPI/焦点探针、托盘与快捷键。
- 岛屿外壳：折叠/展开、顶部居中、跟随显示器变化、贴边隐藏/悬停露出、拖拽、全屏隐藏、设置页入口。
- 设置、主题、i18n：独立设置窗口，明暗主题、中英文切换，设置持久化。
- 剪贴板试点：文本 + 图片历史、收藏、搜索、文本历史持久化。

已完成（2026-11-23）：

- `nsh/host` 编译并运行：AppView 顶层窗口、480×60 / 930×510 物理尺寸（320×40 / 620×340 points @150%）、顶部居中、`WS_EX_TOPMOST`、托盘、`Ctrl+Shift+I/O/M/T`。
- 点击穿透：`Ctrl+Shift+O` 后 `WindowFromPoint` 从 AppView 变为下层窗口，探针确认通过。
- 焦点：启动时 AppView 为前台；`Ctrl+Shift+O` 穿透时调用 `SetFocus(None)`，焦点探针读到 `hwndFocus=0`。
- 剪贴板：文本历史、图片条目、收藏、搜索、设置页与主题/语言切换已接入 `nsh/src`。
- 贴边隐藏：`Ctrl+Shift+T` 收起后窗口中 `y=-38`（125%，露出约 10 points），鼠标移到顶部露出条即恢复。
- 拖拽：`WM_NCHITTEST` 在胶囊左缘返回 `HTCAPTION`，拖动后窗口从 `(1080,10)` 移到 `(1105,30)`。
- 全屏隐藏：全屏窗口在前台时宿主隐藏岛屿，全屏结束后恢复显示。

待完成：


验收：

- 与 Tauri 版并排核对折叠/展开/设置页（目标机器至少一个 125% + 一个 150% 显示器）。
- 置顶、穿透、焦点、托盘、快捷键探针结果稳定；切屏后尺寸与位置正确。
- 穿透焦点策略：进入穿透时释放到最近前台窗口/桌面；岛内输入框有焦点时不误释放；退出只恢复可聚焦状态，不抢焦点。
- 剪贴板文本/图片可显示、收藏、搜索、复制回剪贴板；文本历史重启后仍在。
- `cargo test --manifest-path nsh/Cargo.toml --lib` 通过；`nsh build nsh` 无 TSX/QuickJS bundle；`cargo check`（`nsh/host`）通过；现有 Tauri 版不受影响。

依赖：双显示器验收机；`AppView` 能力声明（`windows.overlay`）；主题与 i18n 的 token/词典映射。

### 穿透焦点策略

- 只有“关闭 → 开启穿透”这一刻执行焦点释放，周期探针不重复释放。
- 焦点在岛内输入框（AppView 的子窗口）时保留焦点并记录，不误释放；焦点在岛屿窗口本身时，优先交回最近的前台窗口，找不到则 `SetFocus(None)`。
- 退出穿透只恢复可聚焦能力，不主动抢回焦点；下一次点击或快捷键再获得焦点。

### Phase 3：业务模块迁移（进行中）

顺序建议（按依赖与风险从低到高）：

1. 剪贴板完整迁移（CF_DIB 写回、旧数据导入、变更监听）。
2. 设置全量迁移（已完成：旧设置/主题导入、独立设置窗口、字段对照表）。
3. AI 状态与 Cindy 通知（已完成，对照表见 [nsh-ai-notifications.md](nsh-ai-notifications.md)）。
4. 媒体播放控制、系统音量、设备电量（已完成核心，对照表见 [nsh-media-and-utilities.md](nsh-media-and-utilities.md)）。
5. 翻译窗口与服务（已完成，见 [nsh-translation.md](nsh-translation.md)）。
6. 快捷启动器（对照表已列，待实现）。
7. 文件中转（核心已完成；实拖/复制成功待解锁复核）。
8. 微信任务栏关注提醒（已完成）；QQ / 邮箱夹具管线已完成，真实客户端/凭据联调待做。
9. 旧外观预设导入（本机不适用；发布前对目标旧库做条件性检查）。

截图套件继续单独立项。

已完成（2026-11-23，剪贴板完整迁移）：

- 文本、图片、收藏、搜索、文本与图片持久化；宿主 `WM_CLIPBOARDUPDATE` 脏标记 + 100 ms 轮询。
- 旧数据导入：读取 Tauri `%APPDATA%\com.focusd.island\clipboard-history.json`，导入文本、收藏和图片（PNG 复制到新目录）；旧文件哈希不变、不删除。
- CF_DIB：图片写回同时有 PNG 与 `Bitmap` / `DeviceIndependentBitmap`，`Clipboard.ContainsImage()` 为 true；Notepad 手动粘贴仍需人眼确认。
- 焦点策略：输入框 `on_focus` / `on_blur` 上报；穿透时输入框有焦点则保留，否则交回最近前台窗口或 `SetFocus(None)`。
- 独立设置窗口：NSH `window()` 创建，齿轮与托盘“设置”都能打开/关闭；含深/浅主题、中/英、剪贴板开关、记录图片、最多保留、清空历史，写入 `settings.json`。
- 旧剪贴板设置：首次运行时从 Tauri `clipboard-history.json` 的 `settings` 读取 enabled / captureImages / maxItems。
- 旧设置与主题：只读导入 WebView2 LevelDB 的 `focusd-island-settings`（按背景亮度映射深/浅主题）和 `focusd-island-locale`（`zh-CN` / `en`），字段白名单与默认值回退见 [nsh-settings-map.md](nsh-settings-map.md)；旧文件不修改、不删除。
- AI 状态与 Cindy 通知：只读 Cindy SQLite，Codex / Claude Code 会话显示运行、完成、需关注三态；四类事件沿用 24 小时窗口与本地毫秒水位，点击聚焦 Cindy，Cindy 在前台时自动算已读。设置页增加 AI 总开关与四类提醒开关，旧 `cindy*` 字段随设置导入。

验收：每个模块有“Tauri 版 vs NSH 版”功能清单；数据可互相读取或有一次迁移；回归页/手动清单覆盖窄窗口与中英文。

依赖：Phase 2 的窗口、设置、存储边界；Win32 辅助模块（钩子、剪贴板监听）在启动器/剪贴板完整迁移前完成。

### Phase 4：硬 Windows 能力与发布（建议 6–10 周，1–2 人）

范围：

- 截图套件单独立项，不并入主线；主线只保留截图入口的占位与协调。
- 打包、签名、自动更新、回滚；先手动更新，稳定后再做静默更新；CI 产出安装包与签名 manifest。
- 性能、内存、启动时间、无障碍、崩溃上报；与 Tauri 版对比。
- 数据迁移工具（应用设置与外观主题、剪贴板历史与收藏）。
- 灰度发布与回退方案。

验收：安装/升级/卸载不破坏用户数据；签名校验通过；更新失败可停留在旧版本；截图各场景在目标机器矩阵上通过。

依赖：Phase 2 的打包链路、Phase 3 的数据迁移；截图可与 Phase 3 并行，但不作为 Phase 3 的阻塞项。

总体量级：1 人约 4.5–7 个月；2 人在 Phase 2 后并行约 3–4 个月。截图套件与外部程序（QQ/微信/邮件）兼容性是最不可控的部分。

## 构建、打包、签名与自动更新

- 构建：`nsh run nsh`（开发，热重载 Rust 改动会重建并重启）；`nsh package --target windows`（发布）。首次 Rust 构建约 60 个 crate；建议固定 `nsh` 版本并在 `nsh.json` 写 `"nsh": ">=0.6, <0.7"`。
- 安装包：输出 `.nsh\out\<Name>\`、`.zip`、`.iss`；安装 Inno Setup 6 后生成 `<Name>-setup.exe`，按当前用户安装到 `%LOCALAPPDATA%\Programs\<Name>`，无需管理员。
- 签名：NSH 不签 Windows 二进制；用 `signtool` 对 `.exe` 和 `setup.exe` 签名，CI 中从密钥库取证书。EV/OV 证书策略与现有发布流程一致。
- 自动更新：NSH 使用自研清单（version/url/sha256/signature/notes）和宿主内更新器；启动/设置页检查，下载 zip、强制 SHA-256、解包、覆盖并重启。便携 zip 已验收；签名仅作元数据，未验证。详见 nsh-release.md。
- 与现有 Tauri 更新不兼容：Tauri 的 `latest.json` 字段/签名与 NSH manifest 不同，不能共用同一文件。迁移期保留旧端点和旧版本更新；新版本使用新端点，或在服务端按 User-Agent/版本分流。
- 数据目录：`nsh.json` 的 `dataDir` 可指到现有 `%APPDATA%\com.focusd.island` 以复用文件；NSH storage 是另一套位置，需明确哪些数据放哪。
- CI：Windows runner + Rust 1.99.1 + NSH 0.6.x + Inno Setup + 签名证书；产物：安装包、zip、`latest.json`、`latest.json.sig`、符号/源码映射（如需要）。

## 对外契约与数据迁移

| 契约 | 迁移影响 | 建议 |
|---|---|---|
| `localStorage`（`focusd-island-*`） | NSH 没有 localStorage；Tauri 的数据在 WebView2 用户目录 | 写一次性导出/导入：从 WebView2 读取或让旧版导出 JSON，新版首次启动导入；保留旧 key 名做映射 |
| 应用 id `com.focusd.island` | NSH 可继续使用同一个 id | 保持，避免任务栏/数据目录/通知身份变化 |
| 开机自启注册表值 `FocuSD Island` | NSH 登录项名称不同 | 迁移时删除旧值并登记新登录项；或继续自写 HKCU Run 保持契约 |
| 凭据管理器条目 | 继续用 Win32 凭据 API 可保持不变 | 保持 key 名与格式 |
| 更新端点 / `latest.json` | 格式不兼容 | 新端点 + 新签名；旧版本继续走 Tauri 更新，设切换版本 |
| Tauri 命令名/事件名/窗口 label | 仅应用内部使用，外部程序不依赖 | 迁移时可改；但插件/外部脚本若引用需另查 |
| 数据目录文件名（如 `legacy-agent-hooks-removed.flag`） | 可保留 | `dataDir` 指向旧目录，继续读写 |

## 风险登记

| 风险 | 影响 | 缓解 |
|---|---|---|
| NSH 0.6.1 仍早期，API 可能变 | 返工 | 固定版本；用 `nsh.json` 版本约束；每阶段升级一次并跑回归 |
| 独立应用拿不到 `HWND` | 逐像素穿透、AppBar、捕获叠加层受限 | 先做辅助窗口；必要时对岛屿切 `AppView` |
| 没有低级键盘钩子/剪贴板事件 | 启动器双击 Ctrl、剪贴板实时性下降 | 自写 Win32 辅助模块；接受轮询延迟 |
| 窗口透明/置顶在游戏、全屏、远程桌面下表现不同 | 遮挡/不显示 | 全屏检测 + 隐藏策略；目标机器矩阵验收 |
| 托盘 SVG 在 Windows 上按模板色渲染 | 品牌色丢失 | 用 PNG/ICO 或实测后调整 |
| Direct2D 排版与 WebView 差异 | 视觉回归、CJK 断行/字重差异 | 设计 token 化；并排截图验收；字体清单固定 |
| 大量 UI 需要重写（无 HTML/CSS） | 工期长 | 先做 Phase 2 外壳与设计系统，再逐模块迁移 |
| 自动更新签名/安装路径变化 | 升级失败、用户数据风险 | 灰度、回滚、保留旧版本安装包；升级前备份 |
| 数据迁移遗漏 | 用户丢设置或剪贴板收藏 | 导出/导入工具 + 迁移测试 + 只读回退 |
| 截图/HDR/混合 DPI | 最高技术风险 | 单独立项，MVP 先行，不阻塞主体 |

## 已确认的决定（2026-11-23，Phase 2 调整）

- **路线**：主体 NSH + 少量 Win32 辅助模块。岛屿必须有 `HWND`，Phase 2 采用自有 Win32 消息循环 + `AppView` 顶层窗口的混合宿主。NSH 的子窗口 `AppView` 会关闭 `transparent`，因此没有采用父子窗口方案；宿主仍完全控制 HWND、消息循环、托盘、快捷键与尺寸。
- **NSH 版本**：锁定 0.6.1，`nsh.json` 写 `=0.6.1`；升级需单独验证。
- **数据迁移**：只迁移应用设置与外观主题、剪贴板历史与收藏。旧待办与笔记文件不读取、不删除、不迁移。字段对照见 [nsh-settings-map.md](nsh-settings-map.md)。
- **自动更新**：接受更换端点与签名；先做手动更新，稳定后再做静默更新，保留一个 Tauri 过渡版本。
- **截图套件**：单独立项，不并入主线。
- **双屏 DPI**：2026-11-23 已在 125%/150% 双屏上验收；代码保留显示器/DPI 变化重新居中。
- **图片写回**：CF_DIB 已在 Phase 3 补上（PNG + DIB）；记事本人工粘贴仍待确认。
- **设置页**：独立设置窗口与旧设置/主题导入已完成；字段对照见 [nsh-settings-map.md](nsh-settings-map.md)，暂无 NSH 等价能力的旧字段保留但不应用。
- **Phase 2/3**：Phase 2 已完成；Phase 3 剪贴板、设置全量迁移、AI 状态与 Cindy 通知已完成；媒体/音量/设备电量已完成；启动器已完成；QQ 真实监听、邮箱真实 TLS 已接，客户端/凭据联调待做；文件中转核心已完成；微信任务栏关注提醒已完成；外观预设本机旧库无数据，发布前做条件性扫描。

## Phase 1 验证记录

- `nsh --version`：0.6.1（2f51d86），安装于 `%LOCALAPPDATA%\nsh`。
- 系统：Windows 11 26H2，build 26300（满足 Windows 10 1809+）。
- `cargo check`（`nsh/`）：通过。
- 当时的 TSX headless 测试（已删除）：1 passed（`opens as a collapsed capsule and expands`）；headless runner 首次构建约 4m26s，增量约 25s。
- `nsh run nsh --target windows`：通过。折叠窗口 320×40、展开 620×340，均位于主显示器顶部居中；点击展开、点 ⌃ 收起均正常。
- 剪贴板：复制唯一文本后，展开窗口通过 UI Automation 读到“1 条”和该文本，点击条目会写回剪贴板。
- 语言：界面默认跟随系统显示中文，点 EN 后全部文案切到英文，按钮变“中文”；托盘 tooltip 同步变为“焦点岛”。
- 托盘：溢出区可见“焦点岛”图标，右键菜单为“显示岛屿 / 分隔符 / 退出”。
- 全局快捷键：Ctrl+Shift+I 注册成功（真实运行无 “taken” 警告），可收起展开窗口。headless 运行会报 taken，是 headless 后端没有全局快捷键支持，不是冲突。
- DPI：主屏 125%（NSH 报 2048×1152 points）、副屏 150%。`displays()` 与 `set_bounds` 都用 points，窗口在 125% 下物理尺寸为 400×50，位置与内容缩放正确；不要用 DPI 未感知的 `GetWindowRect` 结果反推逻辑尺寸。
- 现有 Tauri：未改动 `src/`、`src-tauri/`；`pnpm check` 通过（221 个测试）。

## Phase 2 验证记录（2026-11-23）

- 宿主：`nsh/host`，`cargo check` 通过；`cargo run --manifest-path nsh/host/Cargo.toml` 启动。
- 150% 显示器一次运行：折叠 480×60、展开 930×510 物理像素（320×40 / 620×340 points），顶部居中。
- 125% 显示器一次运行：折叠 400×50、展开 775×425 物理像素，同样居中；说明 DPI 变化由 `GetDpiForWindow` 重算生效。
- 置顶：AppView 窗口带 `WS_EX_TOPMOST`；宿主定时探针未发现层级丢失。
- 穿透：`Ctrl+Shift+O` 后 `WindowFromPoint` 从 AppView 变为下层窗口（`app=false`），再按恢复；穿透时调用 `SetFocus(None)`，焦点探针由 `hwndFocus=app` 变为 `0`。
- 托盘：溢出区“焦点岛”，菜单含显示/收起、切换穿透、下一块屏幕、退出。
- 剪贴板试点：文本历史、图片条目、收藏（`★` 即时更新并写入 `clipboard.json`）、搜索过滤、设置页中文/English 与深/浅主题均通过 UI Automation 验证；浅色主题像素约 `240,240,239`，设置写入 `settings.json`。
- 图片：剪贴板位图规范化为 PNG；Phase 3 已补 CF_DIB 写回（见下）。
- 双屏混合 DPI 已于 2026-11-23 验收，见下方“双屏混合 DPI 验收”。
- 当时的 TSX headless 测试（已删除）：1 passed；`cargo check`（`nsh/host`）通过。
- 现有 Tauri 未改动；`pnpm check` 在 Phase 1 时已通过（221 个测试），本阶段未改 `src/`。
- 贴边隐藏：`Ctrl+Shift+T` 后窗口 y=-38（125%，约露出 10 points），鼠标移到顶部露出条即恢复；托盘和快捷键都能切换。
- 拖拽：胶囊左缘的拖拽条在 `WM_NCHITTEST` 返回 `HTCAPTION`（2），拖动后窗口从 (1080,10) 移到 (1105,30)。
- 全屏隐藏：前台全屏窗口出现时宿主隐藏岛屿，退出全屏后按原尺寸恢复。

## Phase 3 验证记录（剪贴板，2026-11-23）

- 旧数据：从 Tauri 的 `clipboard-history.json` 导入 30 条（22 文本、8 图片）；图片复制到新 `clipboard-images`，旧文件 SHA-256 前后一致，未删除。
- 图片持久化：新 `clipboard.json` 存图片路径，重启后仍能显示；`fs.read` 能力补了 `${dataDir}`，不再出现 not granted 警告。
- CF_DIB：点击图片条目后剪贴板格式为 `PNG, Format17, Bitmap, System.Drawing.Bitmap, DeviceIndependentBitmap`，`Clipboard.ContainsImage()=true`；Notepad 手动粘贴待人工确认。
- 焦点：输入框聚焦时进入穿透，日志为 `island input keeps focus` 且焦点不释放；按 Escape 让输入框失焦后进入穿透，日志为 `keyboard focus cleared`。
- 变更监听：宿主注册 `AddClipboardFormatListener`；复制后 100 ms 内触发读取（日志验证）。

## 双屏混合 DPI 验收（2026-11-23）

显示器（host `log_displays`）：

- 主屏：2560×1440 物理，work 2560×1380，125%，逻辑 2048×1152，原点 (0,0)。
- 副屏：2240×1400 物理，work 2240×1328，150%，逻辑 1493×933，原点 (2560,0)。

结果：

- 切到副屏（Ctrl+Shift+M）：先按旧 DPI 放到 400×50@(3480,10)，DPI 变化后重算为 480×60@(3440,12)；展开 930×510@(3215,12)，均顶部居中。
- 切回主屏：480×60@(1040,12) → 400×50@(1080,10)，尺寸与位置随之重算。
- 置顶：副屏窗口 exstyle 为 `0x200008`（含 `WS_EX_TOPMOST`）；多次切屏后未丢失。
- 穿透：副屏 `Ctrl+Shift+O` 后 `WindowFromPoint` 命中下层窗口（`app=false`），再按恢复 `app=true`。
- 托盘：岛在副屏时点击托盘图标可收起，窗口仍在副屏 `480×60@(3440,12)`；快捷键 I/O/M/T 在副屏均生效。
- 拔插：`DisplaySwitch /internal` 后只剩 2240×1328@150%，岛自动到 `480×60@(880,12)`；`/extend` 恢复双屏后岛保持 `480×60@(3440,12)`。
- 跨屏边缘：把窗口强制放到两屏交界（720×90），宿主按所在屏重新应用为 `480×60@(3440,12)`，不跨屏。
- 缩放修改（副屏）：150%→125%，宿主 144→120 dpi，岛 480×60@(3440,12) → 400×50@(3480,10)；改回 150% 后恢复。
- 缩放修改（主屏）：125%→150%，主屏逻辑 2048×1152→1707×960，岛 400×50@(1080,10) → 480×60@(1040,12)；改回 125% 后恢复 400×50@(1080,10)。

证据：`docs/assets/nsh-dpi/primary-125-collapsed.png`、`docs/assets/nsh-dpi/secondary-150-expanded.png`。

## Phase 3 设置窗口验证（2026-11-23）

- 齿轮与托盘“设置”都能打开独立 NSH 窗口（标题“设置”）；✕ 可关闭，托盘可再次打开。
- 窗口含：深/浅主题、中/英、记录剪贴板 开/关、记录图片 开/关、最多保留 10/20/30/50、清空历史、版本信息。
- 切换浅色后 settings.json 为 dark:false；切换 English 后 locale:en。
- 关闭记录剪贴板后复制测试文本，clipboard.json 数量 28→28；重新开启后 28→29。
- 选择最多保留 10 后下一次复制把历史截到 10 条；测试后恢复 30，并重新从旧历史导入 30 条。
- 旧剪贴板设置（enabled/captureImages/maxItems）从 Tauri `clipboard-history.json` 的 `settings` 读取；旧外观设置从 WebView2 LevelDB 读取，详见下一节。

## Phase 3 设置全量迁移验收（2026-11-23）

**旧数据导入（只读）**

- `focusd-island-settings`：导入 `appearanceMode:"classic"` 与背景 `#000000`，按亮度映射为深色主题；`islandBackgroundColor` 优先于 `taskTextColor`。
- `focusd-island-locale`：本机旧库没有该键（旧版未手动切换过语言），`locale` 回退为空并跟随系统；键存在时只接受 `zh-CN` / `en`。
- 旧剪贴板选项：`enabled=true`、`captureImages=true`、`maxItems=30`，夹在 5–200。
- 首次导入后 `legacy-settings.json` 保存 1954 字节旧设置原文；旧 `clipboard-history.json` 未改动；旧待办/笔记文件没有读取。
- LevelDB 导入后哈希（旧 Tauri 进程仍占用 `.log` / `LOCK`，只记录可读的 `.ldb` 段）：`000005.ldb 45A1FFFC…`、`000008.ldb B00FBEDB…`、`000011.ldb D7B21C70…`；`clipboard-history.json` 为 `5FBA3739…`。
- 字段白名单、默认值回退与删除项见 [nsh-settings-map.md](nsh-settings-map.md)。

**设置窗口与持久化**

- 主题：深色 → 浅色 → 深色，`settings.json` 即时写入；浅色时设置页像素为 `235,235,234`，深色为 `27,29,32`。
- 语言：中文 ↔ English 即时切换整页文案；`settings.json` 的 `locale` 在切到 English 后为 `en`，切回后为 `zh-CN`。
- 备份：每次写入前把上一版复制为 `settings.json.bak`；验证时 `.bak` 内容为切换前的 `dark:false`。
- 重启：结束宿主并重新启动后，岛与设置页仍为中文 + 深色，`settings.json` 的修改时间未变（未重复导入）。
- 状态同步：关闭设置窗口后，岛内入口由 `✕` 恢复为 `⚙`；主题与语言继续生效。
- 多屏：设置窗口移到副屏后 `dpi=144`（150%），窗口 `668×797`，内容边界 `x 2709–3360 / y ≤ 882`，全部落在窗口内、无裁剪；证据 `docs/assets/nsh-settings/secondary-150-light-zh.png`。
- 说明：NSH 窗口跨屏移动时保持原物理尺寸，不按新 DPI 重排（在 150% 屏上显得略小，但布局完整）；窗口再次打开仍在主屏级联位置，这是当前 NSH 行为，不影响使用。
- 证据：`docs/assets/nsh-settings/primary-125-dark-zh.png`、`primary-125-light-zh.png`、`secondary-150-light-zh.png`。

### 双屏探针修复记录（2026-11-23）

**为什么曾只看到一台**：不是 NSH `displays()` 快照，也不是进程 DPI 虚拟化。宿主本身以 PMv2 运行，探针一直用 `EnumDisplayMonitors + GetDpiForMonitor`。之前的“单显示器”记录出现在副屏未接通或执行 `DisplaySwitch /internal` 的时间段；当时的原始日志确实只有一台。

**修复前的失败**：热拔后宿主用 `GetDpiForWindow` 计算尺寸，窗口 DPI 仍停留在旧屏（120），而 NSH 按新屏（144）布局，出现 `client 199x34 != 400x50` 的反复重设；`/extend` 时还会跟随过渡布局短跳。

**修复**：
- 尺寸改按目标显示器的 `GetDpiForMonitor(MDT_EFFECTIVE_DPI)`，不再信任可能滞后的 `GetDpiForWindow`。
- 探针每次显示签名变化都重新枚举并输出原始 `SM_CMONITORS`、`hmon`、`rect`、`work`、`dpi`。
- 显示配置变化后 800ms 内不重排，只按最终布局居中，避免过渡布局跳屏。

**修复后原始输出**：
```text
raw SM_CMONITORS=2
raw display hmon=0xA4E0151 primary=false rect=(2560,0)-(4800,1400) work=(2560,0)-(4800,1328) dpi=144x144 scale=1.50 logical=1493x933
raw display hmon=0x4C90F95 primary=true rect=(0,0)-(2560,1440) work=(0,0)-(2560,1380) dpi=120x120 scale=1.25 logical=2048x1152
```
热拔 `/internal` 后：`SM_CMONITORS=1`，2240×1400@144，岛 480×60@(880,12)；`/extend` 后只重排一次到主屏 400×50@(1080,10)，不再跳屏。

## Phase 3 AI 状态与 Cindy 通知验收（2026-11-23）

- 数据源：只读本机最新 Cindy 数据库（694 MB），查询 `sessions`、`messages`、`schedule_runs`、`subagent_runs`；未写库。
- 三态：实机折叠岛显示“焦点岛 运行 3/4 …”，随会话变化；展开 AI 条显示三态计数和最多 4 个会话行。三态截图用真实 Cindy 库的只读副本，向一个真实 Codex 会话插入一条 `ask_user` 后显示“运行 2 · 需关注 1 · 完成 2”，四行覆盖需关注/运行/完成；原库未写入。
- 已读水位：把 `ai-watermark.json` 回拨 1 小时后重启，折叠岛出现未读角标 “2”；点击“聚焦 Cindy”后前台进程变为 `Cindy`，水位推进到当前读取时间，角标消失。
- 点击失败不标记已读：`focus_cindy()` 返回 false 时不推进水位；Cindy 在前台时轮询会自动推进水位。
- 设置页：新增“AI 状态”区，含总开关、完成/提问/定时与后台/出错四个开关，以及“已连接 Cindy 数据库 · 4 个会话”。关闭总开关后 AI 条消失，恢复后下一次轮询重新出现；四个开关写入 `settings.json` 的 `ai_enabled` 与 `notify_*`。
- 旧设置：`cindyNotificationsEnabled` 与四个 `cindyNotify*` 字段从 WebView2 LevelDB 白名单导入。
- 副屏 150%：设置窗口 `dpi=144`、`668×1022`，内容边界 `x 2709–3360 / y ≤ 1107`，无裁剪。
- 证据：`docs/assets/nsh-settings/ai-panel-125.png`（实机运行态）、`ai-three-states.png`（三态，真实库副本）、`secondary-150-ai-zh.png`；对照表 [nsh-ai-notifications.md](nsh-ai-notifications.md)。
- 验证：`cargo check`（nsh、nsh/host）、`cargo test --lib`（2 passed：三态与“已回答的提问不再算需关注”）、当时的 TSX headless 测试（已删除）、`pnpm check` 均通过。

## 旧外观预设导入（本机不适用）

结论：本机旧库没有自定义外观预设数据，不适用迁移。依据：

- 完整扫描 WebView2 LevelDB 的全部键（`scan_legacy` 读副本），没有 `focusd-island-setting-presets`；键列表见下方记录。
- `%APPDATA%\com.focusd.island` 目录只有剪贴板、歌词缓存、截图、Agent 会话记录与设置窗口位置，没有预设文件。
- [contracts.md](contracts.md) 与 [docs/design/settings.md](../design/settings.md) 记录：样式预设（含主题文件导入导出）已于 2026-11-09 去掉，`focusd-island-setting-presets` 的旧数据保留但不再读写。
- 用户提到的 6 个自定义预设在本机没有对应数据；扫描结果中没有任何 `preset` 键或值。

扫描到的旧 localStorage 键（两个 origin 的并集）：

```text
focusd-island-agent-approval
focusd-island-agent-hidden-sessions
focusd-island-agent-seen-turns
focusd-island-agent-sounds
focusd-island-archives
focusd-island-cindy-notifications-seen
focusd-island-collapsed-text-default-v1
focusd-island-current-date
focusd-island-daily-note
focusd-island-launcher-settings
focusd-island-last-saved-signature
focusd-island-music-sources
focusd-island-qq-notification-access-requested
focusd-island-save-directory
focusd-island-scheduled-todos
focusd-island-settings
focusd-island-todos
focusd-island-translation-settings
```

若发布前在其他目标机器的旧库里发现该键，再按下面的约束补导入：

实现约束：

- 优先只读解析 WebView2 LevelDB：先把 LevelDB 复制到临时目录，再打开副本解析；绝不改动原文件。
- 如果解析不可靠，改为在现有 Tauri 版加一次性“导出外观预设”按钮；这条备选方案要先回报，得到确认后再动 `src/` / `src-tauri/`。
- 导入结果逐项列出：预设名称、颜色、字体、透明度等字段；失败的预设保留在报告里，不静默跳过。
- 现有旧设置原文已备份到 `legacy-settings.json`；预设数据后续单独定位与解析，不覆盖或删除旧数据。

## Phase 3 媒体与设备电量验收（2026-11-23）

- 媒体：没有系统媒体会话时媒体条不占用空间；用临时静音 WAV 打开 Media Player 后，展开岛出现媒体条（标题 IMG_1298 (1)、音量 10%），播放/暂停按钮在 ▶/⏸ 间切换，音量 10% → 30% → 10%，关闭播放器后轮询自动隐藏。
- 设备电量：展开岛显示“🔋 MCHOSE G3 A 11% · keycool 5.0 1% · MCHOSE X75 V2 5.0 100%”，启动时读取并每 60 秒刷新。
- 测试结束后已关闭临时 Media Player、删除临时音频与浏览器配置，并恢复系统音量。
- 提示音：Tauri 有开关和 complete/question/error 三类 WAV；NSH 已复制三份 WAV，PCM 乘 0.7 后用 PlaySoundW 异步播放，设置开关默认开；来源与许可见 [nsh-media-parity.md](nsh-media-parity.md)。
- 验证：cargo check / cargo build（nsh、nsh/host）通过；cargo test --lib 16 passed（1 ignored 的实机外设探针）；当时的 TSX headless 测试 17 passed / 1 skipped（已删除）；媒体与电量均用 UI Automation / PostMessage 实测。

## Phase 3 微信关注提醒验收（2026-11-23）

- 探测：微信运行中，Weixin 主窗口 pid 20376；QQ 未运行；邮箱无 focusd-island-qq-mail-settings 与 com.focusd.island.qq-mail 凭据。
- 宿主注册 SHELLHOOK，只按进程名 Weixin/WeChat 过滤；不读聊天内容。
- 启动后展开岛显示“微信提醒 监听中 打开微信”；向宿主注入 SHELLHOOK/HSHELL_FLASH（目标为真实微信窗口）后变为“需关注”；微信正在后台闪烁，状态与真实事件一致。
- 锁屏状态下聚焦失败，pending 保留；解锁后点击“打开微信”应清除，已列入联调清单。
- QQ/邮箱对照表、失败态与联调清单见 [nsh-messaging.md](nsh-messaging.md)。

## 发布范围总表（判断第一个 NSH 版）

| 模块 | 现版行为 | NSH 现状 | 阻塞发布 | 联调依赖 |
|---|---|---|---|---|
| 剪贴板 | 文本/图片历史、收藏、搜索、旧数据导入 | 已完成 | 否 | 无 |
| 微信 | 任务栏关注提醒，不读正文 | 已完成（主窗口类 mmui::MainWindow） | 否 | 真实消息/解锁下补点击聚焦与开关验收 |
| QQ | 内存通知、点击打开 QQ | 真实 WinRT 监听已接（过滤/未运行态实测）；真实 QQ toast 待登录 | 是 | QQ 登录后联调 |
| 邮箱 | IMAP 邮件头、正文按需、不标已读/不附件/不远程图 | 真实 TLS IMAP 已接；TLS/认证失败实测，有效账号头/正文待测 | 是 | 测试邮箱授权码 |
| 翻译 | 独立窗口、7 家服务、凭据管理器 | 已完成（独立窗口、7 家服务、凭据管理器；mock 验收） | 否 | 真实 Key 未配置 |
| AI 状态 | 三态 + 四类通知 + 声音 | 三态/四类完成；提示音已完成 | 否（提示音已补） | 无 |
| 媒体 | 封面取色、歌词、进度拖动、随机循环、多会话、空闲/均衡器 | 封面/取色、歌词、进度点击/拖动、随机循环、多会话/来源优先级、空闲态、电平字形、提示音完成 | 否 | 已用真实 SMTC 播放器验收 |
| 电量 | BLE/HID/HID++ 外设电量 | 已完成 | 否 | 无 |
| 外观主题 | 深浅/颜色/预设 | 深浅完成；预设本机不适用 | 否（本机） | 目标旧库条件扫描 |
| 设置 | 主题/语言/剪贴板/AI/微信/提示音 | 已完成 | 否 | 无 |
| 快捷启动器 | 应用/文件/URL 搜索启动 | 已完成（应用/UWP/PATH、文件目录、URL、快捷键与设置） | 否 | ↑↓ SendInput 验收，真实键盘待复核 |
| 文件中转 | 拖放、持久化、打开/复制/拖出 | 核心完成：拖放/文件选择/CLI、图标与缩略图、搜索/多选、打开/定位/复制路径（重试）/移除/清空、设置上限与双击/折叠计数；OLE 拖出代码已接 | 是（锁屏下拖出实拖/复制成功待复核） | 解锁后实拖与复制 |
| 截图 | 框选/标注/长截图/OCR/贴图/HDR | S1–S3 完成；混合 DPI/HDR 通过；Chromium 长截图与旧版一致受限；设置页差异已补齐，对丢矩阵无未分类行 | 否 | [nsh-screenshot.md](nsh-screenshot.md) |
| 托盘与快捷键 | 显示/收起/穿透/切屏/贴边 | 已完成 | 否 | 无 |
| 自动更新 | 端点/签名/手动更新 | 核心完成：自研清单、启动检查、下载、SHA-256、解包、覆盖、重启；本地 HTTP 全流程通过；未签名 | 否（未签名风险已提示） | 代码签名/生产 HTTPS |
| 安装包与签名 | NSIS/签名 | 便携 zip 完成；nsh package 生成 .iss，本机未装 Inno Setup，未生成安装器；无签名证书 | 是（安装器/签名未完成） | Inno Setup + 证书 |

媒体验收证据：docs/assets/nsh-media/2026-11-24/（本地 LRC、LRCLIB、KRC 翻译、无歌词、断网、拖动、空闲、多会话/播放器优先级）与 [nsh-media-parity.md](nsh-media-parity.md) 的“2026-11-24 媒体收尾验收”。
发版与更新链路：docs/architecture/nsh-release.md（便携 zip、清单格式、SHA-256、替换/重启、未签名风险、Tauri 过渡与回滚）。

翻译验收证据：docs/assets/nsh-translation/2026-11-24/（中→英、英→中、未配置 Key）与 [nsh-translation.md](nsh-translation.md)；mock 测试用 Rust 本地 TCP 服务（`translation::tests::mock_provider_*`）。

快捷启动器验收证据：docs/assets/nsh-launcher/2026-11-24/（空态、应用、文件、URL、副屏 150%、键盘）与 [nsh-launcher.md](nsh-launcher.md)。

