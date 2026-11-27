# 岛屿 UI parity：把 NSH 外壳做成 Tauri 的样子

NSH 版的岛屿此前是自己一套配色和排版（深灰胶囊 + 文字按钮），和 Tauri 版（`src/windows/island/`）不是同一套视觉。本文记录把 NSH 外壳逐屏向 Tauri 对齐的方案、token 来源和进度。

范围只限外观：窗口尺寸与定位、命中测试与穿透、贴边收起、截图自排除、混合 DPI 与 HDR 的行为都不动。

## 视觉语言从哪来

Tauri 版的样式分散在 `src/styles/tokens.css` 与各组件的 `*.css` 里。NSH 侧把这些值收进 `nsh/src/ui/`：

| 模块 | 内容 |
|---|---|
| `nsh/src/ui/theme.rs` | 几何、颜色、字号、阴影与材质，一项一个常量，注释写明它在 Tauri 里对应哪条规则 |
| `nsh/src/ui/icons.rs` | lucide 图标（与 Tauri 用同一套字形），由 `nsh/scripts/gen-icons.mjs` 从 `lucide-react` 生成，不要手改 |
| `nsh/src/ui/widgets.rs` | 复用的零件：圆形图标按钮、页点、发丝线、进度环、计数徽章、空态文案 |

新写界面用这些名字，不要再写字面量。`palette()` 仍保留给各功能页过渡期使用，但它的取值已经改成 Tauri 的色值，所以旧页面也会自动贴近。

## token 对照

| NSH (`theme::`) | 值 | Tauri 来源 |
|---|---|---|
| `FONT_UI` | Segoe UI Variable Text 栈 | `--font-ui`，`tokens.css` |
| `TEXT_1/2/3/PLACEHOLDER` | `rgba(248,246,240,.96/.68/.54/.42)` | `--text-1` … `--text-placeholder` |
| `ISLAND_BG` | `#000` | `islandBackgroundColor` 默认值，`settingsDefaults.ts` |
| `ISLAND_BG_FALLBACK` | `#2F2F37` | `IslandApp` 舞台兜底色 |
| `COLLAPSED_TOP/BOTTOM` | 白 6% → 黑 22% | `.island__collapsed` 的上下渐变 |
| `CARD_TOP` | 白 4% 渐隐 | 展开卡片顶部的柔光 |
| `RIM` | 白 8% | `--island-rim` |
| `ACCENT` / `SKY` / `SUCCESS` / `DANGER` / `WARNING` | `#74d6ff` / `#a8d8ea` / `#2f8c61` / `#c75065` / `#ffd166` | `--color-accent` 等 |
| `AGENT_IDLE` / `PULSE` / `PULSE_GLOW` | `#35e985` / `#3ae489` / 72% | `--color-agent-idle`，`pulseColor` |
| `DOT_TODO/MUSIC/CLIPBOARD/SHELF/TRANSLATION/BATTERY` | 蓝 / 紫 / 粉 / 灰 / 橙 / 绿 | 页点颜色，`IslandHeader.css` |
| `WHITE_A04…A18` | 白 4%–18% | `--color-white-aNN` |
| `COLLAPSED_HEIGHT` / `COLLAPSED_RADIUS` | 38 / 19 | `.island--collapsed` |
| `EXPANDED_RADIUS` / `PANEL_RADIUS` / `CARD_RADIUS` | 30 / 10 / 8 | `--island-shape-radius` 及其派生 |
| `HEADER_HEIGHT` / `ICON_BUTTON` | 34 / 34 | `.island__header`、`.icon-button` |
| `DOT_WIDTH` / `DOT_CAPSULE_*` | 24 / 18×12 | `.dot-button` 与 `::before` |
| `ISLAND_PADDING_X` / `COLLAPSED_PADDING_X` / `ISLAND_GAP` / `PANEL_PADDING` | 20 / 18 / 14 / 16 | 舞台与面板内边距 |
| `BRAND_SIZE` / `SUMMARY_SIZE` / `BODY_SIZE` / `LABEL_SIZE` / `HINT_SIZE` | 15 / 13 / 13 / 12 / 11 | 与 Tauri 的字号一致 |
| `card_shadow()` | `0 18px 46px rgba(0,0,0,.24)` | 展开卡片的投影（默认玻璃强度） |
| `collapsed_shadow(focused)` | `0 5px 16px rgba(0,0,0,.12)` + 内嵌高光 | `.island--collapsed` 的投影与内描边 |
| `soft_glow()` / `dot_glow()` | 2D 光晕 | 页点与状态点的 `box-shadow` |

## 逐屏进度

| # | 屏幕 | Tauri 参考 | NSH 落点 | 状态 |
|---|---|---|---|---|
| 1 | 收起胶囊 | `IslandCollapsedBar.tsx/.css` | `lib.rs::collapsed_island` | 本轮完成：38px 圆角胶囊、上下渐变、内嵌高光、9px 状态点带光晕、13px 摘要、`flex-1` 弹性留白、12px 计数、chevron 图标；前 8px 是拖拽区 |
| 2 | 展开外壳 | `IslandShell.css` | `lib.rs::expanded_island` | 本轮完成：30px 圆角卡片、顶部柔光、投影、14px 分区间距、20/16 内边距、10px 圆角页面容器 |
| 3 | 页头（标题 / 页点 / 工具按钮） | `IslandHeader.tsx/.css` | `lib.rs::expanded_header` | 本轮完成：15px 粗体标题配强调色图标、三个页点胶囊（悬停放大上浮、选中加长并带光晕）、计数徽章、34px 圆形图标按钮（悬停提亮、按下位移） |
| 4 | 剪贴板列表 | `IslandClipboard*` | `lib.rs::search_and_list` / `history_row` | 待做：目前是纯文字行 + 文字按钮 |
| 5 | 设置页 | `src/components/settings/` | `lib.rs` 设置区 | 待做 |
| 6 | 消息（QQ / 微信 / 邮件） | `IslandMessages*`、`qq_ui.rs`、`mail_ui.rs` | 对应 `*_ui.rs` | 待做 |
| 7 | 电量 | `IslandBattery*` | `lib.rs::battery_panel` | 待做 |
| 8 | 媒体 | `IslandMusic*` | `lib.rs::media_panel` | 待做 |
| 9 | 歌词 | `CollapsedLyric` | `lyrics_ui.rs` | 待做 |
| 10 | 翻译 | `src/components/translation/` | `translation_ui.rs` | 待做 |
| 11 | 快捷启动 | `IslandLauncher*` | `launcher_ui.rs` | 待做 |
| 12 | 文件暂存 | `IslandShelf*` | `file_shelf_ui.rs` | 待做 |
| 13 | 截图工具 | `IslandScreenshot*` | `screenshot_ui.rs` / `screenshot_advanced_ui.rs` | 待做 |
| 14 | 更新提示 | `IslandUpdater*` | `update_ui.rs` | 待做 |

## 本轮已落地

- 新增 `nsh/src/ui/`：token、图标、零件三部分，在 `nsh/src/lib.rs` 里以 `mod ui;` 挂上。
- `palette()` 换成 Tauri 色值（暗色直接用 `theme::` 常量），所以尚未重做的页面配色也随之改变；浅色沿用同一套 alpha 但落在纸白底上。
- 收起、展开、页头三处按上表重写；旧的文字按钮 `EN / ⚙ / ^` 换成图标，`复制 / 收藏` 等页内文字按钮留到各页时改。
- 删掉随之失效的 `ai_dot`、`divider`（新代码内联了它们）。

## 已知差异与取舍

- **模糊与玻璃**：nsh-core 0.7.2 有 `backdrop_blur`，但宿主窗口目前不透明、也没有做 backdrop 采样，本轮只用渐变 + 投影模拟材质。真正的毛玻璃需要改窗口属性，属于行为改动，留到单独一轮。
- **字体**：`FONT_UI` 已定义但还没全局应用，页面会在改造时逐块接上，避免一次性改变所有文案的换行。
- **页点数量**：Tauri 默认 6 个（待办 / 音乐 / 剪贴板 / 暂存 / 翻译 / 电量），NSH 目前只有剪贴板 / 暂存 / 翻译三页，先按已有三页排布；补齐待办与电量页后再对齐。
- **图标**：`icons.rs` 由 lucide 生成后带 `#[allow(dead_code)]`，未用到的图标会在对应页面接上后消失。

## 验证

- 静态：`cargo check --manifest-path nsh/host/Cargo.toml`、`cargo clippy --manifest-path nsh/host/Cargo.toml`（`nsh/src/wechat.rs` 的 `is_multiple_of` 提示是既有问题，不在本次范围）。
- 构建产物：`cargo build --manifest-path nsh/host/Cargo.toml`，运行 `nsh/host/target/debug/focusd-island-host.exe`。
- 界面验收由人工完成：100% / 125% / 150% 三档 DPI 下分别截图收起、展开、各功能页，与 Tauri 版并排比对，截图放 `docs/assets/nsh-ui-parity/<screen>/`。这项工作尚未开始，完成前不得记作已验证。
