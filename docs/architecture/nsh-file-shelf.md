# NSH 文件中转（File shelf）对照与验收

状态：2026-11-25。对应 Tauri 代码为 src/features/file-shelf/ 与 src-tauri/src/file_shelf/。

## 对照表

| 能力 | Tauri 现版 | NSH 当前实现 | 状态 |
|---|---|---|---|
| 添加入口 | 岛内拖放、文件选择、CLI | 宿主 WM_DROPFILES；IFileOpenDialog 多选；CLI 绝对路径 | 完成 |
| 多文件 | 一次加入多个路径 | 同左 | 完成 |
| 非文件拖入 | 文本不进入文件中转 | 只处理文件拖放 | 完成 |
| 存储 | file-shelf.json，最多 100 条 | NSH 数据目录 JSON，原子写 | 完成 |
| 条目 | id/path/name/isDir/size/addedAt/pinned/missing/unreachable | 同左 | 完成 |
| 去重 | 路径大小写/斜杠不敏感，重复移到顶部 | 同左 | 完成 |
| 上限 | 20–200，默认 100；置顶不占上限 | 设置页输入并夹紧 20–200，默认 100 | 完成 |
| 清理 | 按天数/缺失项清理 | 规则已有；当前固定 0 天/移除缺失，设置页未接 | 部分 |
| 列表 | ≤4 卡片，>4 两列行 | 同左；搜索时布局保持 | 完成 |
| 图标 | Explorer Shell 图标 | SHGetFileInfo 图标，按类型/自身缓存 | 完成 |
| 缩略图 | 图片预览；视频走 Shell | 图片/视频 IShellItemImageFactory；图片解码回退；失败回退类型徽标 | 完成 |
| 打开 | 双击/Enter；可配置双击定位 | 双击打开；设置可切“打开/定位” | 完成 |
| 定位 | Explorer 中显示并选中 | explorer.exe /select 并选中 | 完成 |
| 复制 | 复制文件列表 | 复制路径文本（本轮要求）；5 次重试 + 页面重试按钮 | 完成（锁屏下为错误态） |
| 移除/清空 | 单项移除、清空确认 | 单项 ✕、批量移除选中、清空二次确认、设置页清空 | 完成 |
| 拖出 | OLE 拖出文件到 Explorer/聊天窗口 | IDataObject + SHDoDragDrop，经宿主 UI 线程桥接 | 已实现，实拖待解锁复核 |
| 搜索 | 名称/路径过滤 | 同左 | 完成 |
| 多选 | Ctrl 切换、Shift 范围、批量移除 | 同左；选择逻辑有 Rust 单测 | 完成 |
| 重启恢复 | JSON | 同左 | 完成 |
| 大文件 | 只存路径；40 MB 只影响缩略图 | 同左；50 MB 文件正常加入 | 完成 |
| 折叠条 | 可显示文件中转数量 | 可开关，显示“文件 n” | 完成 |
| 鼠标手拖入 | Explorer 拖放 | 只投递 WM_DROPFILES，未做鼠标手拖 | 环境限制 |

## 实机验收（2026-11-25，锁屏远程桌面）

- 拖入：WM_DROPFILES 3 文件（中文名、267 字符长路径、50 MB）→ 岛自动展开；重启后仍在；重复拖入仍 3 条。
- 文件选择：点“添加”打开系统 IFileOpenDialog，实际选中并加入 picker-added.txt；状态 3 → 4。
- CLI：宿主带绝对路径参数启动，自动加入并展开。
- 图标/缩略图：PNG 与 MP4 均生成预览（MP4 由 IShellItemImageFactory，PNG 有解码回退）；TXT 用 Shell 图标；截图 icons-thumbs-125.png。
- 打开：Notepad 打开“中文 报告 2026.txt”；定位：Explorer 打开测试目录并选中该文件。
- 搜索：输入“视频”只剩视频卡片，截图 search-125.png。
- 多选：Ctrl 切换/Shift 范围由 next_selection 单测覆盖；锁屏下合成鼠标不能可靠带修饰键，批量移除用单选中路径验到 3→2。
- 设置：上限输入 5 夹紧到 20，再设 100；双击切“定位”；折叠条计数开关；设置页清空使 JSON 4→0。截图 settings-shelf-125.png。
- 折叠条：开启时显示“文件 2”；关闭后消失；截图 collapsed-count-125.png。
- 复制路径：锁屏且剪贴板被高完整性进程占用时显示“复制路径失败：OSError(5): 拒绝访问。”和“重试”；点重试仍保留列表与错误，不丢 UI。截图 copy-retry-125.png；成功路径待解锁复核。
- 副屏 150%：旧截图 secondary-150.png 记录 930×630、dpi=144。
- 其他截图：features-final-125.png、picker-125.png、icons-thumbs-125.png、search-125.png、copy-retry-125.png、settings-shelf-125.png、collapsed-count-125.png。

## 自动化

- cargo test --lib --offline：109 passed / 9 ignored（含文件中转选择、图标、缩略图、徽标、上限测试）。
- cargo check（nsh 默认与 embedded）、cargo check/build（nsh/host）通过。
- 宿主拖放：SetWindowSubclass + DragAcceptFiles + WM_DROPFILES；拖出/文件选择经 embedded 请求队列在宿主 UI 线程执行。

## 遗留

- 解锁后补：真实鼠标手拖入、真实 OLE 拖出到 Explorer/聊天窗口、复制路径成功读取。
- 清理天数设置页、多选快捷键的实机鼠标组合操作待补。
