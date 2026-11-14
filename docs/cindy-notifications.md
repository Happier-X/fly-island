# Cindy 通知：数据只来自 Cindy 的本地数据库

小岛的通知只有 **Cindy** 一个来源：Cindy 做完了任务、停下来等你回答、跑完定时或后台任务、或者出错时，收起的岛右侧显示一个数字角标，点一下就把 Cindy 的窗口拿到前面。岛上不再列出会话、不再有 Agent 页，也没有状态灯。

## 为什么要读数据库

Cindy 自己有一套「灵动岛」（Electron 内部的 `agent-island:*` 频道），在 macOS 上还会写系统的 Live Activity —— 那是随包的原生 Node 扩展 `cindy-agent-activity.node` 调用 ActivityKit，只在 macOS 27.3 以上加载，Windows 安装包里根本没有这个文件。这条路既不能跨平台复用，也没有对外接口：Cindy 不监听任何端口，`agent-island:*` 全是进程内 IPC，外部程序调不到。

所以本机唯一可用的数据是 Cindy 自己的 SQLite 数据库：`%APPDATA%\Cindy\cindy-<id>.db`（WAL 模式，几百 MB）。岛只做**只读**连接，不改 Cindy 的任何设置、不装钩子、不写它的文件。

## 读什么

`sessions` 表（Cindy 的私有结构，按「列存在才查」写，缺列降级不崩）：

| 列 | 用途 |
|---|---|
| `id` | 通知的指纹 |
| `status` / `hidden_at` / `notifications_muted` | 只看仍在显示、没有静音的活跃会话 |
| `active_turn_started_at` / `last_turn_ended_at` | 这一轮是否已经结束 |
| `source` | 定时任务跑出来的会话算在定时任务里，不在「任务完成」里重复报 |

`messages` 表：最后一条未回滚消息的 `role`；`error` 行是出错通知。Cindy 没有「待处理交互」字段，但回答一定会追加一条 `user` 消息，所以最后一条是 `ask_user` / `plan_review` 就意味着还在等你。查询走 `(session_id, created_at)` 索引，只对还在跑的会话查。

`schedule_runs` 与 `schedules` 表：定时任务的运行记录。Cindy 自己记 `read_at`，只有未读、已结束（`success` / `failed` / `aborted` / `interrupted`）、且任务打开了 `notify_desktop` 的运行才算通知。成功的算「定时与后台任务」，失败的算「出错」。

`subagent_runs` 表：会话里跑的后台子任务。没有已读字段，按结束时间（`ended_at`）落在最近 24 小时内、未删除未回滚、父会话仍活跃的算通知；`completed` 算「定时与后台任务」，`failed` / `stopped` 算「出错」。数据库里没有单独的「后台任务」标记，这是能找到的最接近的记录。

## 四类事件怎么判定

| 类型 | 判定 |
|---|---|
| 任务完成（`done`） | 活跃会话的一轮已经结束：`last_turn_ended_at` 有值、且没有比它更晚的 `active_turn_started_at`；最近 24 小时内 |
| 需要你回答（`question`） | 这一轮还没结束，且最后一条未回滚消息是 `ask_user` / `plan_review`；不设时间窗口，没答就一直算 |
| 定时与后台任务（`scheduled`） | 未读且成功的 `schedule_runs`，或刚结束的后台子任务；最近 24 小时内 |
| 出错（`error`） | 最近 24 小时内新的 `role='error'` 消息，或失败的定时 / 后台任务 |

定时任务跑出来的会话（`source = 'scheduler'`）不参与「任务完成」和消息出错的判定，避免同一次运行报两遍；它的结果只从 `schedule_runs` 来。

## 已读与去重

Cindy 只给定时任务记 `read_at`，其他类型没有已读字段。岛上前端保存一个**本地水位**（Unix 毫秒，`localStorage` 的 `focusd-island-cindy-notifications-seen`）：

- 读到的事件里，比水位新、并且开着对应类型开关的才算未处理；
- 点角标且成功把 Cindy 拿到前台后，或每轮读取时 Cindy 已经在前台，把水位推到这次读取的时间；
- 水位只在变化时写一次（不是每轮写整张表），只增不减；第一次启动从当前时间开始，装好之前发生的事不提醒。

这样不用保存按会话的已读表，也不会因为一条提醒没点而永远算成新的：新事件的时间一定晚于水位。

## 岛上怎么显示

- 收起态右侧原来的「Agent 会话数」位置改成通知角标：有未处理通知时显示铃铛和数量（超过 99 显示 `99+`），点一下把 Cindy 的窗口拿到前台（`focus_agent_window`）；没找到窗口时不清除角标。
- 没有未处理通知时不显示；通知模块关闭时保留原来的灰色收起圆点。
- 不弹面板、不显示会话标题、不列会话。
- 提示音沿用内置的「拇指琴」音效：完成和定时任务用 `complete`，出错用 `error`，提问用 `question`。一次来好几条只响最重要的一条（出错 > 提问 > 完成）。Cindy 已在前台或游戏模式勿扰时不响。设置里只有一个「响提示音」开关。

## 做不到的

- **审批拦截**：Cindy 的授权卡在它自己的界面里，没有可以拦截的 HTTP 钩子，所以岛上没有「允许 / 拒绝」卡片。要审批就在 Cindy 里点。
- **Cindy 的私有表结构**：这些表是它的实现细节，升级可能改列名，代码只按列名试探，缺列时那一类事件降级为空，其余类型照常。读取失败时设置页的「数据来源」会写明原因、路径和最近读取时间。
- **后台任务的边界**：Cindy 没有明确的「后台任务」记录，只能读 `subagent_runs`。它改名或改结构时，这一类会降级为空，其余三类不受影响。
- **旧版本的钩子**：v3.1.0 及以前往 `~/.claude/settings.json` 和 `~/.codex/config.toml` 里写过状态钩子。首次启动新版本时会一次性删掉带 `focusd-agent-` 签名的条目与托管块（用户自己写的保留），记在应用数据目录的 `legacy-agent-hooks-removed.flag`。
