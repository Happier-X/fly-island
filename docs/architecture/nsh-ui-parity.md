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
| 1 | 收起胶囊 | `IslandCollapsedBar.tsx/.css` | `lib.rs::collapsed_island` | 进行中：38px 圆角胶囊、渐变、内嵌高光、9px 状态点带光晕、13px 摘要、计数、chevron 已落地，但**右侧计数与 chevron 被裁到窗口外**（见「阻塞」） |
| 2 | 展开外壳 | `IslandShell.css` | `lib.rs::expanded_island` | 进行中：30px 圆角卡片、顶部柔光、投影、14px 分区间距、10px 圆角页面容器已落地；卡片横向上仍比窗口宽，右侧内容被裁 |
| 3 | 页头（标题 / 页点 / 工具按钮） | `IslandHeader.tsx/.css` | `lib.rs::expanded_header` | 进行中：标题、图标、三个页点已对上；**搜索 / 设置 / 收起三个按钮落在窗口右边界之外**（见「阻塞」） |
| 4 | 剪贴板列表 | `ClipboardHistoryItemRow`、`ClipboardFilters` | `lib.rs::search_and_list` / `history_row` | 本轮完成：32px 搜索框（放大镜图标 + 清空按钮）、44px 行、30px 缩略图/图标、13px 单行预览、发丝分隔线、悬停淡底、26px 圆形复制与收藏按钮。未做：笔记、删除、全部/收藏筛选、面板头、复制成功回钩 |
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

## 阻塞：右对齐控件落到窗口之外

截图证据在 `docs/assets/nsh-ui-parity/round1/`（`dpi125-collapsed.png`、`dpi125-expanded.png`、`dpi150-*.png`）。现象：

- 收起胶囊：状态点、摘要、计数正常，`chevron` 与右侧留白被切掉；计数紧贴右边缘。
- 展开页头：标题与页点正常，右侧工具按钮整组消失，只有最左边的一个图标露出半截。
- 页面正文：电池行、`打开` 按钮等长文本行同样越过右边界被裁。

已试过并且都失败的排布方式（每次都用 PrintWindow 截图确认）：

| 方案 | 结果 |
|---|---|
| 行内 `flex_1` 撑开 + 右侧固定组 | 右侧组被推到窗口外 |
| `justify_between()` 两个组 | 右组完全不可见 |
| `justify_end()` 单子元素行 | 同组内只有第一个子元素可见 |
| `absolute().right()` 锚定 | 展开时宿主进程直接崩溃 |
| 固定宽度 + 前导 `ml()` 绝对偏移 | 组内子元素仍从偏右的位置开始，溢出 |
| 两行叠加（第二行 `mt(-HEADER_HEIGHT)`） | 同上 |
| 单独一行 + `absolute().left(462)` 固定偏移 | 整组完全不渲染 |

### 像素测量（本轮新增的证据）

用 `PrintWindow` 抓图后逐列扫描亮区，得到窗口坐标下的实际落点：

| 屏幕 | 期望 | 实测 |
|---|---|---|
| 收起胶囊：状态点左缘（`ml(18)` + 9px 点） | 18 | 11（点有 2–3px 光晕，实际约 14） |
| 收起胶囊：chevron 右缘（窗口宽 320 − `mr(18)`） | 302 | 317–319（贴到 320 被切） |
| 展开页头：标题图标左缘（`ml(20)`） | 20 | 15 |
| 展开页头：三个工具按钮（`ml(462)`，宽 138） | 462–600 | 徽章 566、搜索 608–619（被切） |

收起胶囊的内容总和比 320 多出约 14px，展开卡片多出约 30px；两者都表现为"外侧容器被撑宽、内容整体左移/右溢出"，而不是某个按钮单独错位。把 `.gap()` 全部换成子元素 `margin` 后总和少了 14px（`when()` 为空的分支也许仍在占一个 gap 槽），但没有根治。

### 两条候选根因（都还没证实）

1. **盒模型**：nsh-core 的 `box_sizing` 只有 BorderBox / ContentBox 两种（`style.rs:3079`），Tailwind preflight 里调的是 `box-border`，理论上 padding 在内。但实测"外层 `.px()` + 固定 `.w()`"的表现更像 padding 加在宽度之外；旧版 `552d74c` 里能用的页头正是 `.px(18)` + `flex_1` 弹簧的组合，和现在的差别只在细节。
2. **DPR / 窗口尺寸**：宿主按 `monitor_dpi / 96` 折算窗口尺寸（`nsh/host/src/island.rs::center_on`），并有一个 watchdog 在客户区尺寸不符时重新 `SetWindowPos`。实测同一进程里窗口出现过 620×420、744×504、384×48 三种尺寸，说明"逻辑布局尺寸"和"实际客户区尺寸"不是一回事；如果 surface 按 620 逻辑点排版、窗口却是 620 物理像素，右侧 1/8 就会落到窗口外——这与"所有贴右的东西都被切"完全吻合。

第 2 条一旦成立就涉及窗口几何与 DPI 折算，属于本任务的禁区（只做视觉），必须先停下来确认再动。

### 最小复现的尝试

做过一次 620×420 的三行对照实验（`w(620)+px(18)` / `w(584)+px(18)` / `w_full()+px(18)`，各带 `flex_1` 弹簧与三个 34px 按钮），但那次窗口自己没有扩展到 620×420（抓到的位图是 408×72，客户区 384×48），实验数据不可用；代码已全部撤回，仓库里没有留下探针。

在解决之前，本轮不宣称任何屏幕"已完成"。

## 验证

- 静态：`cargo check --manifest-path nsh/host/Cargo.toml`、`cargo clippy --manifest-path nsh/host/Cargo.toml`（`nsh/src/wechat.rs` 的 `is_multiple_of` 提示是既有问题，不在本次范围）。
- 构建产物：`cargo build --manifest-path nsh/host/Cargo.toml`，运行 `nsh/host/target/debug/focusd-island-host.exe`。
- 截图：`docs/assets/nsh-ui-parity/round1/` 下的 `dpi125-collapsed.png`、`dpi125-expanded.png`、`dpi150-collapsed.png`、`dpi150-expanded.png`，用 `PrintWindow` 抓宿主窗口（`nsh/host/target/debug/focusd-island-host.exe`，抓前先退出已安装的 0.2.0 以避开单实例互斥，抓完恢复）。
- DPI 说明：本机没有 100% 缩放的显示器，逻辑基准按 620×420 / 320×40 记录，不拍 100% 档。125% 档窗口 620×420；150% 档把窗口移到缩放 150% 的显示器后窗口仍是 620×420（同一进程里也出现过 744×504），**没能验证跨显示器重算缩放**，150% 截图只能说明逻辑布局，不能说明 DPI 缩放。
- 逐屏与 Tauri 版并排比对：未完成。当前差异见上面「阻塞」一节，解决前不记作已验证。
