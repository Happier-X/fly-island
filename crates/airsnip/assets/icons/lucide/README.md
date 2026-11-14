# 截图界面图标（Lucide）

截图覆盖层工具栏、长截图状态条与 OCR 气泡使用的单色矢量图标。素材来自宿主项目已经在用的
**Lucide**（ISC），与岛上 `lucide-react` 图标同一套图形语言：24 单位网格、圆端点、圆连接。

## 生成方式

不要手改这里的 SVG。改完 `scripts/sync-screenshot-icons.mjs` 里的图标表后运行：

```bash
node scripts/sync-screenshot-icons.mjs
```

脚本从已安装的 `lucide-react` 取原始几何，把 `<line>` / `<circle>` / `<rect>` / `<polyline>`
统一写成 `<path d>`，描边写到根节点（`stroke-width="2.25"`，相当于 16px 显示时的 1.5px），
并复制 Lucide 的 `LICENSE`。文件名对应 `src/render/icon/data.rs` 的 `Icon` 变体。

## 渲染约定

- 保留原始 `viewBox="0 0 24 24"`，**不再按墨水包围盒归一化**：短横线若按自身包围盒放大，
  会变成一根粗杠，相邻图标的描边也会粗细不一。
- `src/render/icon/svg/` 在加载时把描边展开成填充轮廓（圆端点、圆连接），
  之后仍走原来的纯填充绘制路径，不增加每帧成本。
- 按钮里按 26px 按钮配 16px 图标框绘制，与岛上 Lucide 的 16px / 1.5px 视觉一致。

## 图标对照

| 文件 | Lucide | 用途 |
|---|---|---|
| `rect.svg` | `rectangle-horizontal` | 矩形工具 |
| `ellipse.svg` | `circle` | 椭圆工具 |
| `arrow.svg` | `move-up-right` | 箭头工具 |
| `line.svg` | `minus` | 直线工具 |
| `freehand.svg` | `pencil` | 手绘工具 |
| `text.svg` | `type` | 文本工具 |
| `highlight.svg` | `highlighter` | 荧光笔 |
| `blur.svg` | `blend` | 高斯模糊 |
| `glow.svg` | `flashlight` | 聚光灯 |
| `number.svg` | `list-ordered` | 序号 |
| `fill.svg` | `paint-bucket` | 填充开关 |
| `undo.svg` / `redo.svg` | `undo-2` / `redo-2` | 撤销 / 重做 |
| `pin.svg` | `pin` | 贴图 |
| `longshot.svg` | `scan-line` | 长截图 |
| `ocr.svg` | `scan-text` | OCR |
| `copy.svg` / `save.svg` | `copy` / `save` | 复制 / 保存 |
| `bold.svg` / `italic.svg` | `bold` / `italic` | 粗体 / 斜体 |
| `minus.svg` / `plus.svg` | `minus` / `plus` | 字号减 / 加 |
| `check.svg` / `cross.svg` | `check` / `x` | 长截图完成 / 取消，气泡关闭 |
| `delete.svg` | `trash-2` | 设置面板清除快捷键 |

设置窗口左侧的「截图」入口仍用同一个 Lucide `Scan`（16px、描边 1.5）。
