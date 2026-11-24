# Tauri 设置 → NSH 设置对照表

旧设置来源（只读）：`%LOCALAPPDATA%\com.focusd.island\EBWebView\Default\Local Storage\leveldb` 中的 `focusd-island-settings` 与 `focusd-island-locale`，以及 `%APPDATA%\com.focusd.island\clipboard-history.json` 的 `settings`。导入前不修改、不删除旧文件；旧设置原文备份到 `%APPDATA%\com.focusd.island.nsh\legacy-settings.json`，每次写入新 `settings.json` 前把上一版复制为 `settings.json.bak`。

## 保留 / 映射

| Tauri 字段 | NSH 设置 | 说明 |
|---|---|---|
| `islandBackgroundColor` / `taskTextColor` | `dark` | 按背景亮度映射深/浅主题；无颜色时回退 `appearanceMode` |
| `clipboard-history` 的 `enabled` | `clipboard_enabled` | 旧历史文件的 `settings` |
| `clipboard-history` 的 `captureImages` | `capture_images` | 同上 |
| `clipboard-history` 的 `maxItems` | `max_items` | 夹在 5–200 |
| `focusd-island-locale` | `locale` | 只接受 `zh-CN` / `en`；缺失或无效时跟随系统，设置窗口可切换 |
| `cindyNotificationsEnabled` | `ai_enabled` | AI 状态与 Cindy 通知总开关；缺失按开 |
| `cindyNotifyDone` / `cindyNotifyQuestion` / `cindyNotifyScheduled` / `cindyNotifyError` | `notify_done` / `notify_question` / `notify_scheduled` / `notify_error` | 四类事件提醒，缺失按开 |

已有 NSH `settings.json` 的升级路径：首次启动时用 `legacy-ai-settings-imported.flag` 标记，一次性把 `cindy*` 五个字段补进来，不覆盖其他已保存的设置。

## 删除（待办/笔记相关）

| Tauri 字段 | 原因 |
|---|---|
| `todoEnabled`、`todoBackgroundColor` | NSH 版不实现待办 |
| `showIdleTodoCount`、`carryOverIncompleteTodos`、`enableTodoReorder` | 待办能力整体取消 |
| `rotateTodos`、`todoRotationSeconds` | 待办轮播取消 |
| `pageOrder` / `collapsedPriority` 中的 `todo` 项 | 页面与折叠栏不再有待办 |
| 笔记、归档相关设置 | NSH 版整体不实现 |

## 新增（NSH 设置窗口）

| 设置 | 说明 |
|---|---|
| 深色 / 浅色 | 岛屿与设置窗口即时切换并持久化 |
| 中文 / English | 运行时切换全部文案 |
| 记录剪贴板 开/关 | 立即停止/恢复采集 |
| 记录图片 开/关 | 只影响图片条目 |
| 最多保留 10/20/30/50 | 下一次采集时截断 |
| 清空历史（保留收藏） | 保留收藏项 |
| 独立设置窗口 | 普通可缩放窗口，支持多屏；齿轮与托盘入口 |

## 暂不迁移（NSH 当前无等价能力）

外观预设（名称、颜色、字体、透明度等）：本机旧库没有 `focusd-island-setting-presets`，也不存在预设文件，判定为“不适用”；依据与目标机器上的条件性检查见 [nsh-migration.md](nsh-migration.md) 的“旧外观预设导入”。

| 旧字段组 | 处理 |
|---|---|
| `appearanceMode`、`fontFamily`、`glassIntensity`、`opacity`、`collapsedScale`、`expandedScale`、`marginY`、`islandMonitor`、`islandCornerRadius`、`islandCornerShape`、`maxPanelHeight`、`positionLocked` | 保留旧值但不应用；当前主题只有深/浅，尺寸固定在宿主 |
| `lyricsFolder`、`lyricsOnline`、`lyricsChinese` | 已接入 NSH 设置；首启从旧设置导入，之后保存在 nsh settings.json |
| `lyricsOffsetMs`、`chineseOffset` | 不迁移为全局偏移；改为每首歌的偏移，存在 nsh 数据目录 lyrics-offsets.json |
| `agent*` | AI 状态迁移时再接 |
| `music*`（播放器优先级） | 已接入：settings.json 的 music_sources 存 id/label 顺序；其余 music* 仍未接 |
| `shelf*` | 文件中转迁移时再接 |
| `translation*` | 已接入：provider/model/方向/快捷键/开关从旧 LevelDB 导入并写入 settings.json；Key 仍在 Windows 凭据管理器 |
| `mail*`、`battery*`、`game*` | 对应功能迁移时再接 |
| `showTitle`、`showCollapsedText`、`collapsedLayout`、`collapsedShowCover`、`expandTrigger`、`hover*`、`autoTuck*`、`autoCollapse*`、`minimizeAction`、`tuckedHoverAction`、`revealOnMessage` | 岛屿行为迁移时再接；当前宿主只提供手动贴边/展开 |

## 验收

- 旧设置导入后，设置窗口再次打开不丢项：主题、语言、剪贴板开关/图片/上限都来自新 `settings.json`；旧 LevelDB 与 `clipboard-history.json` 的哈希不变。
- 修改主题/语言/外观即时生效并持久化。
- AI 总开关与四类提醒开关导入后重新打开设置不丢项，并即时影响角标。
- 设置窗口在副屏 150% 下布局正常，可缩放、可移动到另一块屏。
- 关闭设置窗口后，岛内齿轮状态恢复，主题/语言继续生效。
