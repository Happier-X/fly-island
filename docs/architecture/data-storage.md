# 数据存放位置

| 数据 | 位置 | 负责方 |
|---|---|---|
| 设置、预设、各功能开关、语言 | WebView2 的 `localStorage` | 前端各 feature 的 `storage.ts` |
| 锁定小岛位置、独立设置窗口 | 同一 WebView2 `localStorage`：`focusd-island-settings.positionLocked`（默认 false）、`focusd-island-detached-settings`（缺失时默认 `"true"`，保留已保存的选择） | `features/appearance/storage.ts`、`windows/settings/storage.ts` |
| 待办、以后的待办、每日笔记、归档、正在进行的待办、待办日期 | WebView2 的 `localStorage`（`focusd-island-todos` 等 key，见 [contracts.md](contracts.md)） | `features/todos/storage.ts` |
| Cindy 通知的水位、旧钩子清理标记 | 水位在 WebView2 的 `localStorage`（`focusd-island-cindy-notifications-seen`，见 [contracts.md](contracts.md)）；应用数据目录的 `legacy-agent-hooks-removed.flag` 记录旧钩子已清理 | `features/cindy-notifications/storage.ts`、`cindy_notifications/legacy_hooks.rs` |
| Cindy 通知的数据来源 | 只读 `%APPDATA%\Cindy\cindy-*.db`（`sessions`、`messages`、`schedule_runs` / `schedules`、`subagent_runs`），不写入 Cindy 的任何文件 | `cindy_notifications/cindy.rs` |
| 文件中转列表（只存路径，不复制文件） | 应用数据目录（`file-shelf.json`） | `file_shelf` |
| 设置窗口上次关闭时的大小与位置（位置已不在任何屏幕上时忽略） | 应用数据目录（`settings-window.json`） | `settings_window/bounds.rs` |
| QQ 邮箱授权码、翻译 API 密钥 | Windows 凭据管理器（`com.focusd.island.qq-mail`、`com.focusd.island.translation`；翻译密钥按服务分条目，用户名为服务 id，DeepSeek 沿用 `deepseek`） | `qq_mail`、`translation` |
| 待办 Markdown 导出 | 用户选择的目录（经过校验） | `system` |
| 开机自启 | `HKCU\...\Run` 下的值 `FocuSD Island` | `system` |
| 剪贴板历史、通知、邮件正文 | 仅保存在内存中 | 各服务模块 |

总览见 [overview.md](overview.md)。
