# NSH 文件中转（File shelf）对照与验收

状态：2026-11-24。对应 Tauri 代码为 src/features/file-shelf/ 与 src-tauri/src/file_shelf/。

## 对照表

| 能力 | Tauri 现版 | NSH 当前实现 | 状态 / 差异 |
|---|---|---|---|
| 添加入口 | 岛内拖放、文件选择、CLI 参数 | 宿主在 AppView 上 DragAcceptFiles + WM_DROPFILES；只接受文件路径 | 拖入完成；文件选择/CLI 未接 |
| 多文件 | 一次加入多个路径 | 一次取全部 HDROP 路径 | 完成 |
| 非文件拖入 | 文本不进入文件中转 | 只处理 WM_DROPFILES，文本不进入 | 完成 |
| 存储 | file-shelf.json，最多 100 条，原子写 | NSH 数据目录 com.focusd.island.nsh/file-shelf.json，最多 100 条，临时文件 + rename | 完成 |
| 条目字段 | id/path/name/isDir/size/addedAt/pinned/missing/unreachable | 同左；missing/unreachable 读取时计算 | 完成 |
| 去重 | 路径大小写与斜杠不敏感，重复项移到顶部 | path_key 同规则；重复拖入不增加条目 | 完成 |
| 数量上限 | 默认 100，可设 20–200；置顶不占上限 | 默认 100；normalize 支持 20–200 与置顶豁免 | 设置页未接，当前用默认 100 |
| 清理 | 可按天数清理、可移除缺失项 | 支持天数/缺失项规则；当前固定 0 天、移除缺失 | 设置页未接 |
| 列表布局 | ≤4 个大卡片，>4 两列紧凑行 | ≤4 卡片，>4 两列行 | 完成 |
| 图标 | Explorer Shell 图标、图片缩略图 | 类型徽标（PDF/DIR/TXT…）+ 文件名/大小/路径 | 未接 Shell 图标与缩略图 |
| 打开 | 双击/Enter 打开；可配置双击定位 | 单击选中，450ms 内双击打开；行内 ↗ 打开 | 无“双击定位”设置 |
| 定位 | Explorer 中显示并选中 | ShellExecuteW 调 explorer.exe /select,"path" | 完成，已验选中中文文件 |
| 复制 | 复制文件列表（CF_HDROP） | 复制路径文本（用户本轮要求） | 行为有意不同 |
| 移除/清空 | 单项移除，清空需确认 | 单项 ✕；清空二次确认 | 完成 |
| 拖出 | OLE 拖出文件 | 未实现 | 后续打磨 |
| 置顶 | ★，置顶不占上限、不自动清理 | ★ 置顶，规则同左 | 完成 |
| 搜索/多选 | 搜索、Ctrl/Shift 多选 | 未实现 | 后续打磨 |
| 重启恢复 | 读取 JSON | 启动读取 JSON，重启后恢复 | 完成 |
| 大文件 | 只存路径，无文件大小上限；>40 MB 不做缩略图 | 只存路径，无大小限制；无缩略图 | 50 MB 文件已验，正常加入 |
| 失败态 | 缺失/不可达、命令错误 | 缺失/不可达标记；打开/定位/复制错误显示在页面底部 | 完成 |
| 折叠条 | 可显示文件中转数量 | 未加折叠条数量 | 后续打磨 |

## 实机验收（2026-11-24）

环境：Windows 11，主屏 125%，副屏 150%；宿主 nsh/host 嵌入 NSH AppView。

- 拖入：向 AppView 发送真实 WM_DROPFILES（3 个路径：中文名、267 字符长路径、50 MB 文件），宿主日志 file drops: 3 files，岛自动展开到 775×525（125%）。
- 持久化：关闭并重启宿主，3 条仍从 file-shelf.json 恢复，计数“3 个文件”。
- 去重：同一组 3 个路径再次拖入，仍是 3 条，顺序移到顶部。
- 大文件：large-50mb.bin 52428800 字节，正常加入并显示 50 MB；Tauri 的 40 MB 只是缩略图源限制，不是加入限制。
- 打开：点击中文文件的 ↗，Notepad 打开“中文 报告 2026.txt”。
- 定位：点击 ⌖，Explorer 打开 focusd-shelf-test 并选中“中文 报告 2026.txt”（Shell.Application 读取确认）。
- 复制路径：点击 ⧉ 后页面显示“复制路径失败：OSError(5): 拒绝访问。”；当前桌面锁屏且剪贴板所有者为高完整性进程 ego，OpenClipboard 返回 access denied。成功路径待解锁后复核。
- 移除：点击一个 ✕，计数从 3 → 2，JSON 同步为 2。
- 清空：点“清空”再点“确认清空”，计数 0，JSON 为空，显示空态。
- 副屏：宿主切到 150% 显示器，日志 monitor_dpi=144 ... 930x630，截图 secondary-150.png。
- 截图：docs/assets/nsh-file-shelf/2026-11-24/main-125.png、secondary-150.png、empty-125.png、copy-denied-125.png。

## 自动化

- cargo test --lib --offline：106 passed / 9 ignored（含文件中转的去重/置顶/上限/大小与徽标测试）。
- cargo check（nsh，默认与 embedded）通过；cargo check / cargo build（nsh/host）通过。
- 宿主拖放实现：SetWindowSubclass + DragAcceptFiles + WM_DROPFILES，路径入 embedded 队列，由 App 引擎线程 80ms 轮询加入并打开文件中转页。

## 遗留

- 当前桌面锁屏，未做鼠标手拖；已投递 Explorer 拖放同一条 WM_DROPFILES，宿主实际收到 3 个路径。解锁后可补一次鼠标手拖。
- 复制路径成功路径需在解锁桌面、同完整性剪贴板环境下再验一次。
- 拖出（OLE）、Shell 图标/图片缩略图、文件选择、CLI、搜索/多选、折叠条数量、设置页的 100/20–200 与自动清理尚未接。
