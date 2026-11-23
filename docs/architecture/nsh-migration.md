# FocuSD Island → NSH 迁移评估（Phase 1）

> 状态：Phase 1 可行性 spike 与评估。现有 Tauri 实现不改动，`nsh/` 与新文档独立提交。
> 验证环境：Windows 11 26H2（build 26300）、NSH 0.6.1（2f51d86）、Rust 1.99.1。

## 结论

**可行，但不能整体平移。** NSH 已经能支撑岛屿的外壳与大部分业务界面：无边框、透明、半透明、`aboveDock` 置顶、动态改宽高、托盘、全局快捷键、i18n、剪贴板读取都在 `nsh/` spike 中跑通，Rust root 不需要 QuickJS。

主要受限点是 **Windows 原生能力**：`nsh run` 的 Rust root 不暴露自己的 `HWND`，也没有低级键盘钩子、剪贴板变更通知、系统媒体会话、UIA/窗口读取、屏幕捕获与 OCR、凭据管理器。Phase 2 已用“自有 Win32 消息循环 + `AppView` 顶层窗口”拿到 HWND；其余能力仍需继续用 `windows` crate 自写。截图套件（HDR、长截图、贴图、混合 DPI）风险最高，已改为单独立项。

推荐路线：**Rust root 的纯 NSH 应用**，把现有 `src-tauri` 的领域模块逐步改成可复用的 Cargo crate；需要 `HWND` 的能力先用独立 Win32 辅助窗口实现，只有在“逐像素点击穿透 / AppBar / DWM 自绘 / 捕获叠加层”成为硬需求时，才升级为 `AppView` 混合宿主。

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
| 自动更新 | 有，但格式不同 | `nsh.json` 的 `updater` + `nsh update-manifest` 生成签名 `latest.json`；Rust 应用要直接用 `nsh-updater` crate 并自带 HTTP 传输。与现有 Tauri 更新端点/格式不兼容。 |
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
| 自动更新 | `nsh package` + `nsh update-manifest` / GitHub Releases；Rust 应用用 `nsh-updater` 校验签名、下载、替换、重启。 | 中高：签名密钥、manifest 格式、安装目录和旧 Tauri 版本升级路径都要重新设计。 |
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

**建议先走 A**，把截图、全局钩子、剪贴板监听做成独立的 Win32 辅助模块；只有当实际验收发现“必须主窗口 `HWND`”时，再针对岛屿窗口切到 C。不要一开始就选 C，否则等于把 Tauri 的窗口层重写一遍再叠加 NSH。

## Phase 2–4 里程碑

### Phase 2：宿主、外壳与基础设施（进行中，建议 5–7 周，1 人）

范围：

- 混合宿主：自有 Win32 消息循环、`AppView` 顶层窗口、HWND 控制、置顶/穿透/多屏 DPI/焦点探针、托盘与快捷键。
- 岛屿外壳：折叠/展开、顶部居中、跟随显示器变化、贴边隐藏/悬停露出、拖拽、全屏隐藏、设置页入口。
- 设置、主题、i18n：岛内设置页，明暗主题、中英文切换，设置持久化。
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

- 双显示器混合 DPI 实测（当前会话只有一个活跃显示器，分别见过 150% 与 125%；代码按 `GetDpiForWindow` 重算并在显示器变化时重新居中）。
- 设置窗口独立化（当前为岛内设置页；独立窗口延后）。

验收：

- 与 Tauri 版并排核对折叠/展开/设置页（目标机器至少一个 125% + 一个 150% 显示器）。
- 置顶、穿透、焦点、托盘、快捷键探针结果稳定；切屏后尺寸与位置正确。
- 剪贴板文本/图片可显示、收藏、搜索、复制回剪贴板；文本历史重启后仍在。
- `nsh test nsh` 通过；`cargo check`（`nsh/host`）通过；现有 Tauri 版不受影响。

依赖：双显示器验收机；`AppView` 能力声明（`windows.overlay`）；主题与 i18n 的 token/词典映射。

### Phase 3：业务模块迁移（建议 7–10 周，1–2 人）

顺序建议（按依赖与风险从低到高）：

1. AI 状态 / Cindy 通知（只读数据 + 展示）。
2. 剪贴板完整迁移（文件、拖出、变更监听；试点已在 Phase 2）。
3. 设置全量迁移（所有开关、数字、快捷键、颜色）。
4. 翻译窗口、快捷启动器（含双击 Ctrl 的 Win32 钩子）。
5. 媒体控制、外设电量。
6. 微信 / QQ / 邮件通知与附件。
7. 文件中转（拖入/拖出）。

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
- 自动更新：`nsh.json` 配 `updater.github` 或 `updater.url + publicKey`；`nsh update-manifest` 生成签名 `latest.json`，与各平台压缩包一起上传。Rust 应用要用 `nsh-updater` crate：`Config::check` 校验 manifest，`Download` 校验大小/SHA-256，`Installation::install` 替换，`restart()` 重启。TSX 部分可走 OTA 签名包；Rust 代码变更必须发新安装包。
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
- **数据迁移**：只迁移应用设置与外观主题、剪贴板历史与收藏。旧待办与笔记文件不读取、不删除、不迁移。
- **自动更新**：接受更换端点与签名；先做手动更新，稳定后再做静默更新，保留一个 Tauri 过渡版本。
- **截图套件**：单独立项，不并入主线。
- **Phase 2**：已启动；顺序为宿主/外壳 → 设置/主题/i18n → 剪贴板试点（图文历史、收藏、搜索）。

## Phase 1 验证记录

- `nsh --version`：0.6.1（2f51d86），安装于 `%LOCALAPPDATA%\nsh`。
- 系统：Windows 11 26H2，build 26300（满足 Windows 10 1809+）。
- `cargo check`（`nsh/`）：通过。
- `nsh test nsh`：1 passed（`opens as a collapsed capsule and expands`）；headless runner 首次构建约 4m26s，增量约 25s。
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
- 图片：剪贴板位图被规范化为 PNG 后显示；点击图片条目写回 `PNG` 格式（.NET 读到 412 字节 PNG），不是 `CF_DIB`；旧式应用可能需要 Phase 3 的 Win32 DIB 写入。
- 当前会话只有一个活跃显示器（先 150%、后 125%），未能做同一时刻的双屏混合 DPI；代码在显示器/DPI 变化时会重新居中，需在双屏机器上补验。
- `nsh test nsh`：1 passed；`cargo check`（`nsh/host`）通过。
- 现有 Tauri 未改动；`pnpm check` 在 Phase 1 时已通过（221 个测试），本阶段未改 `src/`。
- 贴边隐藏：`Ctrl+Shift+T` 后窗口 y=-38（125%，约露出 10 points），鼠标移到顶部露出条即恢复；托盘和快捷键都能切换。
- 拖拽：胶囊左缘的拖拽条在 `WM_NCHITTEST` 返回 `HTCAPTION`（2），拖动后窗口从 (1080,10) 移到 (1105,30)。
- 全屏隐藏：前台全屏窗口出现时宿主隐藏岛屿，退出全屏后按原尺寸恢复。
