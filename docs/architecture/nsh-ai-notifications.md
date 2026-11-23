# AI 状态与 Cindy 通知：Tauri → NSH 对照

数据源和判断口径以现有 Tauri 版为准：只读 Cindy 的本地 SQLite（`%APPDATA%\Cindy\cindy-*.db` 中最新的一份），不写库、不改旧数据。NSH 侧实现在 `nsh/src/ai.rs`，每 5 秒轮询一次。

## 生命周期与三态

| 维度 | Tauri 现有行为 | NSH 实现 | 结果 |
|---|---|---|---|
| 会话来源 | `sessions` 表；`status='active'`、`hidden_at IS NULL`、`notifications_muted=0`、`source<>'scheduler'` | 同样的过滤条件；缺列时降级为空 | 一致 |
| 会话范围 | 最近 2 小时有活动的会话（`list_agent_sessions`） | 同样 2 小时窗口，最多 30 条 | 一致 |
| 运行 | `active_turn_started_at > COALESCE(last_turn_ended_at, 0)` | 同一表达式 | 一致 |
| 需关注 | 最后一条消息是 `ask_user` / `plan_review`；错误消息单独计通知 | 最后一条消息是 `ask_user` / `plan_review`，或最后一条是 `role='error'` | 一致（错误并入需关注态） |
| 完成 | 本轮已结束；旧版状态灯在 10 分钟内显示 completed | 本轮已结束且 `last_turn_ended_at` 在 10 分钟内 | 一致 |
| Codex / Claude Code | 旧版会话页只有 `provider = cindy`；DB 的 `agent_kind` 区分执行器 | 按 `agent_kind` 显示：`cc` / `claude` → Claude Code，`codex` → Codex，其他原样显示 | NSH 更明确 |
| 会话标题 | `title`，缺失时项目目录 | `title`；缺失时显示 `Agent` | 基本一致（项目目录回退未做） |
| 点击行为 | `focus_agent_window`：把 Cindy 主窗口带到前台；找不到窗口时返回 false | 同左；点击 AI 摘要、会话行或“聚焦 Cindy”都调用同一逻辑 | 一致 |
| 找不到来源窗口 | 不标记已读，角标继续 | `focus_cindy()` 返回 false 时不推进水位 | 一致 |

三态优先级：需关注 > 运行 > 完成。折叠岛的圆点颜色和文字按这个优先级显示（例如有需关注时只显示“需关注 N”，不再显示运行数）。

## 通知事件与已读水位

| 维度 | Tauri 现有行为 | NSH 实现 | 结果 |
|---|---|---|---|
| 事件类型 | `done` / `question` / `scheduled` / `error` | 同一组四类，事件字段为 `kind`、`key`、`at` | 一致 |
| 完成事件 | 24 小时内结束的活跃会话，排除定时任务会话 | 同一 SQL 条件 | 一致 |
| 提问事件 | 仍在运行且最后一条消息是 `ask_user` / `plan_review` | 同左 | 一致 |
| 错误事件 | 24 小时内 `role='error'`，过滤已回滚、已删除、静音会话 | 同左 | 一致 |
| 定时任务 | `schedule_runs.read_at IS NULL`、成功/失败/中止/中断；尊重 `notify_desktop` | 同左 | 一致 |
| 后台任务 | `subagent_runs` 的 `completed` / `failed` / `stopped`，过滤回滚/删除 | 同左 | 一致 |
| 已读去重 | 本地毫秒水位；只显示比水位新的事件；首次启动从当前时间开始 | `%APPDATA%\com.focusd.island.nsh\ai-watermark.json`，规则相同 | 一致 |
| Cindy 在前台 | 新事件直接推进水位，不打扰 | 轮询时检测前台进程名 `cindy`，命中则推进水位 | 一致 |
| 点击后已读 | 只有成功把 Cindy 带到前台才推进水位 | 同左 | 一致 |
| 设置开关 | 总开关 + 四类事件开关，默认全开 | 同一组开关，写入 NSH `settings.json` | 一致 |
| 声音 | 有提示音和静音设置 | 暂未实现，只有角标 | 推迟 |
| 通知弹窗 | 无独立 toast，依靠角标 | 同左 | 一致 |

## 展示

- 折叠岛：圆点按三态着色；有 AI 会话时在标题后显示“运行 N / 需关注 N / 完成”；有未读通知时显示数字角标；保留剪贴板条数。
- 展开岛：标题下增加 AI 条，显示三态数量、最多 3 个会话行（状态、Codex/Claude Code、标题），右侧“聚焦 Cindy”。点击摘要、会话行或“聚焦 Cindy”都会聚焦 Cindy。
- 设置页：AI 总开关、完成/提问/定时与后台/出错四类提醒开关、数据源状态（数据库已连接 / 未找到 / 读取失败原因）。
- 旧设置导入：`cindyNotificationsEnabled`、`cindyNotifyDone`、`cindyNotifyQuestion`、`cindyNotifyScheduled`、`cindyNotifyError` 从 WebView2 LevelDB 的白名单字段读入，缺失按 `true`。

## 差异与限制

- 不做会话级深链：Cindy 所有会话在同一个窗口，Tauri 也只聚焦应用窗口；NSH 保持一致。
- 不重新安装 Claude Code / Codex 钩子：与 v3.4.0 之后的 Tauri 行为一致，旧钩子仍由旧版清理。
- 提示音和声音设置未迁移；通知以角标为主。
- 标题缺失时不回退到工作目录，只显示 `Agent`。
- 会话列表只显示最近 2 小时内有活动、且处于需关注/运行/10 分钟内完成的会话；24 小时事件仍参与角标。
