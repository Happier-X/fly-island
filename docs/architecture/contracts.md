# 对外契约清单

纯重构中下列标识不得改变，规则见根目录 [AGENTS.md](../../AGENTS.md#对外契约不得随意变更)。新增契约时同步更新本节。

## localStorage key

| key | 当前定义位置 |
|---|---|
| `focusd-island-settings` | `features/appearance/storage.ts` |
| `focusd-island-collapsed-text-default-v1`（迁移标记） | `features/appearance/storage.ts` |
| `focusd-island-setting-presets`（2026-11-09 起不再读写：样式预设已去掉，旧数据留着不动） | — |
| `focusd-island-todos` | `features/todos/storage.ts` |
| `focusd-island-active-todo` | `features/todos/storage.ts` |
| `focusd-island-current-date` | `features/todos/storage.ts` |
| `focusd-island-archives` | `features/todos/storage.ts` |
| `focusd-island-scheduled-todos`（安排在以后日期的待办，`{ id, title, date, createdAt }[]`） | `features/todos/storage.ts` |
| `focusd-island-daily-note` | `features/todos/storage.ts` |
| `focusd-island-save-directory` | `features/todos/storage.ts` |
| `focusd-island-last-saved-signature` | `features/todos/storage.ts` |
| `focusd-island-position` | `windows/island/storage.ts` |
| `focusd-island-music-sources` | `features/media/storage.ts` |
| `focusd-island-settings-focus`（主窗口 → 设置窗口的跳转请求） | `windows/settings/storage.ts` |
| `focusd-island-detached-settings`（`"true"` / `"false"`：设置在独立窗口打开；2026-11-10 起没有或无效时默认独立窗口，已保存的 `"false"` 仍为岛上内嵌，不进外观预设） | `windows/settings/storage.ts` |
| `focusd-island-position-reset`（设置窗口 → 主窗口：让岛回到主屏顶部居中，`{ requestedAt }`，超过 5 秒的请求忽略） | `windows/settings/storage.ts` |
| `focusd-island-panel-preview`（设置窗口 → 主窗口：拖动尺寸滑块时让岛展开或收起预览，`{ mode, requestedAt }`，超过 2 秒的请求忽略） | `windows/settings/storage.ts` |
| `focusd-island-qq-mail-settings` | `features/messages/mail/storage.ts`（`{ accounts: [{ email, host, enabled }] }`，兼容读取旧的单账户 `{ email, enabled }`，解析在 `mailAccounts.ts`） |
| `focusd-island-qq-mail-remote-images`（2026-11-11 起不再读写：邮件里的远程图片总是加载，已存的值留在原处不删） | 无 |
| `focusd-island-qq-notifications-enabled` | `features/messages/qq-notifications/storage.ts` |
| `focusd-island-qq-notification-access-requested` | `features/messages/qq-notifications/storage.ts` |
| `focusd-island-wechat-reminders-enabled` | `features/messages/wechat/storage.ts` |
| `focusd-island-translation-settings` | `features/translation/storage.ts` |
| `focusd-island-agent-approval`（2026-11-12 起不再读写：Cindy 的授权卡在它自己的界面里，岛上不再拦截审批；旧值留在原处不删） | — |
| `focusd-island-agent-hidden-sessions`（2026-11-14 起不再读写：Agent 页已删除，旧值留在原处不删） | — |
| `focusd-island-agent-seen-turns`（2026-11-14 起不再读写：按会话的已读表换成下面的通知水位，旧值留在原处不删） | — |
| `focusd-island-cindy-notifications-seen`（`{ watermark }`：Cindy 通知的本地水位，Unix 毫秒；比它新的事件才算未处理，只增不减，只在变化时写） | `features/cindy-notifications/storage.ts` |
| `focusd-island-agent-sounds`（`{ enabled }`：提示音开关；旧版存的音量、音效包、事件和静音时段不再读取，键名沿用） | `features/cindy-notifications/sounds/storage.ts` |
| `focusd-island-agent-session-filters`（2026-11-09 起不再读写：隐藏会话规则已去掉，旧规则不再生效；格式曾为 `[{ kind: "folder" | "prompt", value }]`） | — |
| 外观主题文件（2026-11-08 起；`fly-island-theme-<日期时间>.json`，写到「下载」文件夹，命令 `save_theme_file`，Rust 端按文件名前缀和内容放行；`{ app: "fly-island", kind: "theme", version: 1, name, exportedAt, appearance }`，`appearance` 只含 `features/appearance/settings.ts` 的 `APPEARANCE_SETTING_KEYS`；导入时只读这些字段，叠在当前设置上经 `normalizeSettings` 校验后存成一个新的样式预设，不自动应用） | `features/appearance/theme.ts` |
| `focusd-island-locale` | `i18n/index.ts` |
| `focusd-island-update-channel`（`"stable"` / `"beta"`；3.0.2 及更早的设置写入，现已不读，一律按稳定版检查） | — |

## 事件

| 事件名 | 发送方 | 目标 |
|---|---|---|
| `island-position-changed` | `island/mod.rs` | 广播（通过主窗口对象调用 `emit`） |
| `island-position-cleared`（无载荷：岛屿跟随鼠标或前台窗口换了显示器，拖动保存的位置作废，主窗口清掉 `focusd-island-position`） | `island/follow.rs` | 主窗口 |
| `island-top-attachment-changed`（载荷为 `bool`：岛屿是否贴住所在显示器顶边） | `island/top_edge.rs` | 广播（通过主窗口对象调用 `emit`） |
| `island-hover-changed`（载荷为 `bool`：光标是否在岛屿形状内，供悬停展开使用） | `island/passthrough.rs` | 广播（通过主窗口对象调用 `emit`） |
| `island-cursor-moved`（载荷 `{ x, y }`：光标在主窗口 CSS 像素中的位置，窗口外也有；只在 `set_island_cursor_tracking(true)` 后、光标移动时发，约 28 次/秒，供中心图案的眼睛跟随） | `island/passthrough.rs` | 广播（通过主窗口对象调用 `emit`） |
| `foreground-fullscreen-changed`（载荷为 `{ active, onIslandMonitor }`：前台是否有全屏程序、是否占住岛所在的显示器，供游戏模式使用） | `system/fullscreen.rs` | 广播 |
| `media-session-changed`（无载荷：某个媒体会话的时间线、播放状态或歌曲变了，或会话增减；主窗口收到后立即重读 `get_media_state`。2026-11-11 起） | `media_control/watcher.rs` | 广播 |
| `clipboard-history-changed` | `clipboard_history/service.rs`（常量在 `mod.rs`） | 广播 |
| `clipboard-history-shortcut` | `clipboard_history/service.rs`（常量在 `hotkey.rs`） | 广播 |
| `clipboard-history-shortcut-status`（载荷为字符串或 `null`：剪贴板快捷键注册失败的原因，成功或关闭时为 `null`；最近一次结果也可用 `get_clipboard_shortcut_error` 读取） | `clipboard_history/hotkey.rs` | 广播 |
| `qq-mail-changed` | `mail/service.rs`（常量在 `mod.rs`） | 广播 |
| `qq-notifications-changed` | `qq_notifications/mod.rs` | `main`、`settings` |
| `wechat-reminder-changed` | `wechat_reminders/mod.rs` | `main`、`settings` |
| `translation-window-opened` | `translation/mod.rs` | 广播 |
| `translation-island-shortcut`（翻译设为「岛内页面」时按下翻译快捷键：先唤出主窗口，再让它切换翻译页） | `translation/mod.rs` | 广播 |
| `settings-open-requested`（无载荷：托盘、翻译窗口或独立设置窗口要打开设置，主窗口按 `focusd-island-detached-settings` 决定打开岛上的设置页还是独立窗口） | `settings_window/mod.rs`、前端 `windows/settings/api.ts` | `main` |
| `focusd-update-state` / `focusd-update-request`（更新状态由主窗口广播；独立设置窗口用后者请求 `snapshot` / `check` / `install` / `dismiss`；3.0.2 及更早还有 `channel`） | `features/updater/updaterController.ts` | 广播 / `main` |

## 其他

- 窗口 label：`main`、`translation`、`settings`；前端分发参数为 `?window=translation`、`?window=settings`。
- Tauri 命令：以 `lib.rs` 中 `generate_handler!` 列表为准，命令名即 Rust 函数名。
- 托盘语言命令 `set_tray_language`：参数 `{ locale: "zh-CN" | "en" }`，主窗口首次加载及应用语言变化时同步；沿用 `focusd-island-locale` 与现有系统语言检测，不另存原生语言设置。托盘创建时先用中文，主窗口加载后应用解析出的语言。
- 应用标识 `com.focusd.island`，凭据服务名 `com.focusd.island.qq-mail`、`com.focusd.island.translation`。
- 注册表值：`HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 下的 `FocuSD Island`。
- 旧版本写入的 Agent 钩子（2026-11-12 起不再安装）：`~/.claude/settings.json` 里带 `focusd-agent-` 签名的条目、`~/.codex/config.toml` 里 `# BEGIN/END FocuSD Agent Status Hooks` 之间的托管块。新版首次启动时只删这些，用户手写的内容保留，删完在应用数据目录写下 `legacy-agent-hooks-removed.flag`（同一份目录下已有同名文件时不再动那两个配置文件）。
- Cindy 通知的数据来源（2026-11-14 起）：只读 Cindy 的本地 SQLite 数据库 `%APPDATA%\Cindy\cindy-*.db`（`sessions`、`messages`、`schedule_runs` / `schedules`、`subagent_runs`），见 [docs/cindy-notifications.md](../cindy-notifications.md)。
- 更新端点与 `latest.json` 格式：见 `tauri.conf.json` 和 `docs/updater.md`。
- Cindy 通知命令（2026-11-14 起）：`get_cindy_notifications`（返回 `{ items: [{ kind, key, at }], databasePath, checkedAt, error }`；`kind` 为 `done` / `question` / `scheduled` / `error`，`databasePath` 找不到时为空字符串，读不出来时 `error` 带原因而不是命令失败）、`focus_agent_window`（把 Cindy 的窗口拿到前台，返回是否找到）、`agent_window_in_front`（Cindy 是否就在前台）。旧的 `get_agent_status`、`list_agent_sessions` 与音效包命令 `list_sound_packs` / `import_sound_pack` / `remove_sound_pack` / `read_sound_pack_file` / `open_sound_pack_folder` 已删除；更早的钩子、审批与额度命令见上一节的历史记录。
- 歌词缓存（2026-11-06 起）：应用数据目录下的 `lyrics-cache/<FNV-1a 哈希>.json`（`{ key, lrc, source?, translation?, wordTimes?, durationMs?, durations?, fetchedAt }`；`lrc` 可带增强 LRC 的逐字时间 `<mm:ss.xx>`（酷狗 KRC 转来）；`wordTimes` 为 true 的条目由保留逐字时间的版本写入，没有它的酷狗旧条目会重新下载；`durations` 为 true 的条目由记录歌曲时长的版本写入（`durationMs` 是来源给的时长，可为空），没有它的旧条目会重新查一次（2026-11-10）；`lrc` 为 null 表示所有在线来源都没有，7 天后重查；`source` 为 `lrclib` / `kugou` / `netease`，没有时是 LRCLIB 的旧条目；开着中文来源时 key 末尾多 `cn`；最多 2000 个文件）。设置项 `lyricsFolder`、`lyricsOnline`（2026-11-10 起默认开，保存过 false 的仍为关）、`lyricsChinese`（也查酷狗和网易云，同样默认开）、`lyricsOffsetMs`。每首歌自己的偏移存在 localStorage `focusd-island-lyrics-offsets`（`{ <歌名歌手时长/2 秒>: 毫秒 }`，±10 秒，最多 500 首）。媒体状态多了 `trackId`：播放器插件写进流派的歌曲 ID（`NCM-<id>`、`QQ-<id>`），`get_lyrics` 的查询多了 `chinese`、`trackId`，返回值多了 `translation`，2026-11-10 起还有可选的 `durationMs`（来源给的歌曲时长，播放器不报告时长时用于进度条）。
- 岛屿设置 `focusd-island-settings` 2026-11-10 新增三个字段：`collapsedPriority`（`["lyrics","todo"]` 的某种顺序，收起的岛先显示哪一样；2026-11-14 起去掉了 `"agent"`）、`collapsedShowCover`（布尔，默认 false）、`pageOrder`（`["todo","music","clipboard","shelf"]` 的某种顺序，展开岛顶栏的页面圆点）；同日又加 `collapsedLayout`（`["text","mail","shelf","music","notifications"]` 的某种顺序，收起的岛上文字和状态图标从左到右的位置，`text` 之前的图标在文字左边；旧数据里的 `"agents"` 读成 `"notifications"`）。读取时去掉未知和重复项，缺的按默认顺序补在末尾。
- 岛屿设置 `focusd-island-settings` 的 `pageOrder` 2026-11-11 起多一个可选项 `"translation"`（翻译的页面胶囊），默认顺序 `["todo","music","clipboard","shelf","translation"]`；旧数据缺它时按上面的规则补在末尾，旧版本读到它会当未知项丢掉。
- 岛屿设置 `focusd-island-settings` 2026-11-11 新增 `autoCollapseSeconds`（3 / 5 / 10 / 30 / 0，0 表示从不；展开的岛闲置多少秒后自动收起）。不在这几个值里、或旧数据没有这个字段时：`autoTuck` 为 true 读成 30，否则读成 0。
- 岛屿设置 `focusd-island-settings` 的 `expandedScale`（展开后的大小）2026-11-10 重新可以设置；2026-11-11 起是 0.85–1.2 之间的任意值（滑块，步长 0.01）。3.0.0 存的 0.9 / 1 / 1.1（小 / 中 / 大）原样保留；更早版本存的任意数（原范围 0.75–1.4）读取时限制到 0.85–1.2 并取到百分位，非数字按 1。字段名和类型不变。
- 岛屿设置 `focusd-island-settings` 的 `gameDoNotDisturb`（布尔，默认 true）2026-11-11 重新可以设置（通用 → 岛屿 → 游戏模式）：只有明确存为 false 才关；`gameAutoTuck` 仍按默认 false 读取。字段名和类型不变，没有新 key。
- 岛屿设置 `focusd-island-settings` 新增 `positionLocked`（布尔，默认 false）：只在明确保存为 true 时禁止拖动。旧数据缺失或类型无效时保持可拖动。`set_island_interaction` 增加可选参数 `positionLocked`，缺失时按 false；`start_island_drag` 在锁定时不启动拖动。复位和贴边隐藏不受影响。
- Agent 页显示方式（2026-11-07 起，2026-11-14 起不再读写）：`agentSessionView`、`agentArchiveMotion`、`agentCardEffects` 等只服务 Agent 页的字段已不再读取，旧值留在原处不删。
- 岛屿设置 `focusd-island-settings` 2026-11-14 起与通知相关的字段：`cindyNotificationsEnabled`（总开关）、`cindyNotifyDone`、`cindyNotifyQuestion`、`cindyNotifyScheduled`、`cindyNotifyError`（四类事件，均为布尔，默认 true）。旧的 `agentStatusEnabled`、`agentTitleEnabled` 不再读取。
- 回复状态标记（2026-11-12 起不再读写）：不再往 Claude 的规则文件里写任何内容，「等你回答」改由 Cindy 的 `ask_user` / `plan_review` 消息判断。仍有 `~/.claude/rules/focusd-island-reply-marker.md` 的机器可以手动删除。
- Agent 页排列（2026-11-08 起，2026-11-14 起不再读写）：`agentSessionSort`、`agentSessionGroup` 等随 Agent 页一起删除，旧值留在原处不删。

总览见 [overview.md](overview.md)。
