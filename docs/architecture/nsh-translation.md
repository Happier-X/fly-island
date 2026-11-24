# 翻译窗口：Tauri → NSH 对照与验收（2026-11-24）

## 对照表

| 能力 | Tauri 现版 | NSH 现状 | 结论 |
|---|---|---|---|
| 入口 | 主岛翻译按钮、全局快捷键、可切到岛内页面 | 展开岛头部“剪贴板 / 翻译”页签 + 全局快捷键；页签打开岛内页，快捷键打开独立窗口 | 对齐 |
| 窗口 | 480×640，无边框、透明、可调整、始终置顶、跳过任务栏，首次按鼠标所在显示器居中 | NSH 独立窗口 480×640，无边框、透明、可调整、AboveDock、跳过任务栏，居中并恢复上次位置 | 对齐 |
| 快捷键 | 默认 Ctrl+Shift+F，可改；关闭翻译时注销 | Win32 RegisterHotKey 独立消息线程；设置里点“录制”后按组合键即写入，Esc 取消；被岛屿固定快捷键占用时拒绝并提示；有重置按钮 | 对齐（录制 + 冲突 + 重置） |
| 方向 | auto / zh-en / en-zh；auto 按中文字符与拉丁词比例判断 | 同左，逻辑直接移植 direction.rs | 对齐 |
| 剪贴板 | 打开时填充空输入；Tab/按钮从剪贴板粘贴 | 打开时自动填充空输入；按钮从剪贴板粘贴 | 对齐 |
| 复制译文 | 复制按钮，显示“已复制” | 同左，使用 arboard | 对齐 |
| 空输入/超长 | 空输入、5000 字符上限 | 同左，前后端都限制 | 对齐 |
| 错误态 | 缺 Key、缺模型、超时、HTTP 错误、解析失败 | 同左；30s 超时，错误原文显示，重试按钮复用提交 | 对齐 |
| Provider | DeepSeek、火山方舟、通义千问、Kimi、硅基流动、OpenAI、自定义 OpenAI 兼容 | 同 7 家，base_url/model 规则照 provider.rs；DeepSeek 默认 deepseek-flash，其余必须填模型，custom 只允许 https 或本机 http | 对齐 |
| 密钥 | Windows 凭据管理器，服务 com.focusd.island.translation，按服务 id 分条目 | Win32 CredRead/CredWrite/CredDelete，target 为 provider.com.focusd.island.translation，与 keyring 旧条目同名，无需复制 | 对齐 |
| 旧设置迁移 | localStorage focusd-island-translation-settings | 首次启动从旧 WebView2 LevelDB 只读副本导入，写入 settings.json；旧 island settings 的 translationEnabled 一并导入 | 已实现；本机旧值 provider=siliconflow、无模型 |
| 旧密钥迁移 | keyring 条目 | 直接读取同名旧条目；读不到就显示“未配置 API 密钥，去设置” | 已实现 |
| 自动翻译 | 停止输入 500ms 后自动提交，可开关，默认关 | 同一套 500ms 防抖；设置里开关，默认关；窗口与岛内页共用 | 对齐 |
| 岛内翻译页 | 方向、原文、译文、粘贴/清空/复制/翻译 | 展开岛“翻译”页签：方向栏、原文、译文、粘贴/清空/复制/翻译，和窗口共用状态与设置 | 对齐 |

## 实现

- 核心：nsh/src/translation/{mod,direction,provider,secret,hotkey}.rs。
- 窗口与设置：nsh/src/translation_ui.rs；展开岛“翻译”页签和独立窗口共用状态；设置窗口新增“快捷翻译”分组。
- 自动翻译：输入停止 500ms 后提交，手动提交过的同一文本不重复；方向/Provider/模型改变后允许重新自动提交。
- 快捷键录制：设置行内的只读输入接收按键，组装标签后经 hotkey::set_shortcut 注册；Ctrl+Shift+I/O/M/T 被视为岛屿占用并提示，重置回 Ctrl+Shift+F。
- 凭据：secret.rs 的 target 生成与 keyring 的 Windows store 一致（user + "." + service），旧 Key 可直接读取；不写日志、不写 settings.json。
- 请求：reqwest，30s 超时；UI 在 blocking 任务里运行私有 tokio runtime，结果用 channel 回传，不阻塞 NSH 引擎线程。
- 设置：settings.json 增加 translation_enabled、translation_shortcut、translation_direction、translation_provider、translation_custom_base_url、translation_models。

## 验收

- cargo test --lib --offline：66 passed / 0 failed / 8 ignored。
- mock：node nsh/tests/mock-openai.mjs（127.0.0.1:8787）。
  - translation::tests::mock_provider_translates_both_ways 通过：中文自动到英文、英文自动到中文。
  - translation::tests::mock_provider_reports_401_and_timeout 通过：401 含 HTTP 401；300ms 超时返回错误。
  - translation::tests::a_closed_port_is_a_network_error 通过：端口关闭返回网络错误。
- UI 实测（真实 NSH 窗口）：
  - docs/assets/nsh-translation/2026-11-24/zh-to-en.png：剪贴板“你好”，auto 方向，结果 Hello from mock。
  - docs/assets/nsh-translation/2026-11-24/en-to-zh.png：剪贴板 Hello，结果“你好，来自 mock”。
  - docs/assets/nsh-translation/2026-11-24/no-key.png：未配置 Key 的引导态。
  - docs/assets/nsh-translation/2026-11-24/island-auto.png：岛内翻译页，剪贴板“你好”在自动翻译开启时直接得到 Hello from mock。
  - docs/assets/nsh-translation/2026-11-24/settings.png：设置里的自动翻译开关、快捷键录制/重置、方向、7 家服务、模型与密钥。
- 快捷键：向快捷键线程投递 WM_HOTKEY 后翻译窗口正常打开；录制按键的标签转换与宿主冲突有单测。验收时桌面锁屏，录制按钮的真实点击未完成。
- 旧 Key：旧环境没有 com.focusd.island.translation 条目；测试用 custom 临时 Key 验证 target 名为 custom.com.focusd.island.translation，随后已删除。因此没有真实 Provider 请求，只报 mock 结果。
- 旧设置：只读 LevelDB 中读到 “Ctrl+Shift+F / auto / siliconflow / 无模型”，导入逻辑已运行并写 legacy-translation-settings-imported.flag。

## 遗留

- 真实 Provider 请求需用户配置 Key，本机未配置。
- 验收桌面锁屏，快捷键录制的真实按键未人工复核；单测覆盖标签转换与 Ctrl+Shift+I/O/M/T 冲突。