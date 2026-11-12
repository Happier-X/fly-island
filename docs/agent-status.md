# AI Agent 状态：数据来源是 Cindy

小岛的 AI Agent 区显示的是 **Cindy** 里的会话。Cindy 是本机唯一运行 agent 会话的程序，所以这里只有它一个来源，没有别的 agent 接入。

## 为什么要读数据库

Cindy 自己有一套「灵动岛」（Electron 内部的 `agent-island:*` 频道），在 macOS 上还会写系统的 Live Activity（苹果灵动岛）——那是随包的原生 Node 扩展 `cindy-agent-activity.node` 调用 ActivityKit，只在 macOS 27.3 以上加载，Windows 安装包里根本没有这个文件。这条路既不能跨平台复用，也没有对外接口：Cindy 不监听任何端口，`agent-island:*` 全是进程内 IPC，外部程序调不到。

所以本机唯一可用的数据是 Cindy 自己的 SQLite 数据库：`%APPDATA%\Cindy\cindy-<id>.db`（WAL 模式，525 MB 上下）。岛只做**只读**连接，不改 Cindy 的任何设置、不装钩子、不写它的文件。

## 读什么

`sessions` 表（Cindy 的私有结构，按「列存在才查」写，缺列降级不崩）：

| 列 | 用途 |
|---|---|
| `id` / `title` / `working_dir` | 会话标识、标题、项目名 |
| `model` | 卡片上的模型名 |
| `status` / `hidden_at` | 只列 `active` 且未隐藏的 |
| `active_turn_started_at` / `last_turn_ended_at` | 判断这一轮是否还在跑 |
| `last_activity_at` | 多久前有动静、列表排序 |
| `list_preview` | 卡片上最近一条消息 |
| `context_tokens` / `context_window` | 卡片右端的上下文大小 |
| `total_token_usage` / `total_cost_usd` | 这个会话累计的 token 与花费 |
| `permission_mode` | 自动 / 跳过确认 / 计划 等标记 |

「等你回答」靠 `messages` 表推断：最后一条消息的 `role` 是 `ask_user` 或 `plan_review` 就说明它停在问你。回答一定会追加一条 `user` 消息，所以这个判定不会一直挂着。查询走 `(session_id, created_at)` 索引，只对正在跑的会话查。

## 岛上怎么显示

- 状态灯：有会话在等你或在跑 → 运行中；最近 10 分钟内刚跑完 → 已完成；否则空闲。
- Agent 页：最近 2 小时有动静的会话，在跑和等你回答的排前面；点一行把 Cindy 的窗口拿到前台（所有会话都在 Cindy 一个窗口里，所以没有「找哪个终端」的问题）。
- 声音与弹出：只在完成、出错、有人问你时响 / 弹；Cindy 已在前台时不弹。等待要持续 3 秒才提醒，同一次等待只提醒一次。

## 做不到的

- **审批拦截**：Cindy 的授权卡在它自己的界面里，没有可以拦截的 HTTP 钩子，所以岛上没有「允许 / 拒绝」卡片。要审批就在 Cindy 里点。
- **Cindy 的私有表结构**：`sessions` 是它的实现细节，升级可能改列名，代码只按列名试探，不假设顺序。读取失败时岛只是不显示 AI 区，设置页的「数据来源」会写明原因。
- **旧版本的钩子**：v3.1.0 及以前往 `~/.claude/settings.json` 和 `~/.codex/config.toml` 里写过状态钩子。首次启动新版本时会一次性删掉带 `focusd-agent-` 签名的条目与托管块（用户自己写的保留），记在应用数据目录的 `legacy-agent-hooks-removed.flag`。
