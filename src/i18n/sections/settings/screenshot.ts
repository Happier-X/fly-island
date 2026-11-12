export type SettingsScreenshotText = {
  title: string;
  shortcut: string;
  pressKeys: string;
  saveTitle: string;
  saveHint: string;
  saveFormat: string;
  formatPng: string;
  formatJpeg: string;
  formatWebp: string;
  saveDir: string;
  saveDirDefault: string;
  saveDirReset: string;
  chooseDir: string;
  colorsTitle: string;
  colorsHint: string;
  colors: string;
  colorLabel: (index: number) => string;
  colorsReset: string;
  overlayKeysTitle: string;
  overlayKeysHint: string;
  toolKeys: string;
  functionKeys: string;
  pressKey: string;
  keyUnbound: string;
  keysReset: string;
  toolbarTitle: string;
  toolbarHint: string;
  toolbarToggle: (label: string, visible: boolean) => string;
  interfaceTitle: string;
  ocrLanguage: string;
  ocrLanguageAuto: string;
  ocrLanguageFailed: string;
  uiScale: string;
  uiScaleAuto: string;
  /** 覆盖层元素的叫法；工具栏结构与键名来自 Rust，这里只负责翻译。 */
  slotLabels: Record<string, string>;
  groupLabels: Record<string, string>;
  tryTitle: string;
  tryNow: string;
  tryNowHint: string;
  tryNowAction: string;
};

export const zhCN: SettingsScreenshotText = {
  title: "截图",
  shortcut: "截图快捷键",
  pressKeys: "按下组合键",
  saveTitle: "保存",
  saveHint: "格式和目录用于「另存为」对话框的默认值；直接复制到剪贴板不受影响。",
  saveFormat: "默认保存格式",
  formatPng: "PNG",
  formatJpeg: "JPEG",
  formatWebp: "WebP（无损）",
  saveDir: "默认保存目录",
  saveDirDefault: "系统图片文件夹下的 airsnip 目录",
  saveDirReset: "恢复默认",
  chooseDir: "选择",
  colorsTitle: "标注颜色",
  colorsHint: "覆盖层工具栏上的 5 个色块，改成常用的颜色即可。",
  colors: "色板",
  colorLabel: (index) => `第 ${index} 个颜色`,
  colorsReset: "恢复默认色",
  overlayKeysTitle: "覆盖层快捷键",
  overlayKeysHint: "截图窗口打开后才生效：直接按这些键切换工具。点按键可改绑，录制时按 Backspace 清除、Esc 取消。",
  toolKeys: "工具",
  functionKeys: "功能",
  pressKey: "按任意键",
  keyUnbound: "未设置",
  keysReset: "恢复默认键",
  toolbarTitle: "工具栏",
  toolbarHint: "关掉的元素不会出现在截图工具栏上，键位和快捷键仍然可用。",
  toolbarToggle: (label, visible) => `${visible ? "隐藏" : "显示"}${label}`,
  interfaceTitle: "识别与界面",
  ocrLanguage: "OCR 识别语言",
  ocrLanguageAuto: "跟随系统",
  ocrLanguageFailed: "读取系统语言包失败，暂时只能跟随系统。",
  uiScale: "覆盖层缩放",
  uiScaleAuto: "自动（按显示器）",
  slotLabels: {
    "tool.rect": "矩形",
    "tool.arrow": "箭头",
    "tool.text": "文本",
    "tool.highlight": "荧光笔",
    "tool.blur": "模糊",
    "tool.glow": "聚光灯",
    "tool.number": "序号",
    "tool.line": "直线",
    "tool.ellipse": "椭圆",
    "tool.freehand": "手绘",
    "color.1": "颜色1",
    "color.2": "颜色2",
    "color.3": "颜色3",
    "color.4": "颜色4",
    "color.5": "颜色5",
    "edit.fill": "填充",
    "edit.undo": "撤销",
    "edit.redo": "重做",
    "edit.text_row": "字体行",
    "output.pin": "贴图",
    "output.longshot": "长截图",
    "output.ocr": "OCR",
    "file.copy": "复制",
    "file.save": "保存",
  },
  groupLabels: {
    tools: "工具",
    colors: "颜色",
    edit: "编辑",
    output: "输出",
    file: "文件",
  },
  tryTitle: "试一下",
  tryNow: "立即截图",
  tryNowHint: "唤起全屏覆盖层，框选后可以标注、复制或另存。按 Esc 退出。",
  tryNowAction: "开始截图",
};

export const en: SettingsScreenshotText = {
  title: "Screenshots",
  shortcut: "Capture shortcut",
  pressKeys: "Press keys",
  saveTitle: "Saving",
  saveHint: "These fill in the save dialog's defaults; copying straight to the clipboard is unaffected.",
  saveFormat: "Default format",
  formatPng: "PNG",
  formatJpeg: "JPEG",
  formatWebp: "WebP (lossless)",
  saveDir: "Default folder",
  saveDirDefault: "The airsnip folder inside Pictures",
  saveDirReset: "Use default",
  chooseDir: "Choose",
  colorsTitle: "Annotation colors",
  colorsHint: "The five swatches on the overlay toolbar; swap them for the colors you use.",
  colors: "Swatches",
  colorLabel: (index) => `Color ${index}`,
  colorsReset: "Reset colors",
  overlayKeysTitle: "Overlay keys",
  overlayKeysHint: "They work while the capture window is open. Click a key to rebind it; press Backspace to clear or Esc to cancel while recording.",
  toolKeys: "Tools",
  functionKeys: "Actions",
  pressKey: "Press a key",
  keyUnbound: "Unbound",
  keysReset: "Reset keys",
  toolbarTitle: "Toolbar",
  toolbarHint: "Hidden items leave the capture toolbar; their keys still work.",
  toolbarToggle: (label, visible) => `${visible ? "Hide" : "Show"} ${label}`,
  interfaceTitle: "Recognition and scale",
  ocrLanguage: "OCR language",
  ocrLanguageAuto: "Follow the system",
  ocrLanguageFailed: "Could not read the installed language packs; only the system default is offered.",
  uiScale: "Overlay scale",
  uiScaleAuto: "Automatic (per display)",
  slotLabels: {
    "tool.rect": "Rectangle",
    "tool.arrow": "Arrow",
    "tool.text": "Text",
    "tool.highlight": "Highlighter",
    "tool.blur": "Blur",
    "tool.glow": "Spotlight",
    "tool.number": "Number",
    "tool.line": "Line",
    "tool.ellipse": "Ellipse",
    "tool.freehand": "Pen",
    "color.1": "Color 1",
    "color.2": "Color 2",
    "color.3": "Color 3",
    "color.4": "Color 4",
    "color.5": "Color 5",
    "edit.fill": "Fill",
    "edit.undo": "Undo",
    "edit.redo": "Redo",
    "edit.text_row": "Text row",
    "output.pin": "Pin",
    "output.longshot": "Long shot",
    "output.ocr": "OCR",
    "file.copy": "Copy",
    "file.save": "Save",
  },
  groupLabels: {
    tools: "Tools",
    colors: "Colors",
    edit: "Edit",
    output: "Output",
    file: "File",
  },
  tryTitle: "Try it",
  tryNow: "Take a screenshot",
  tryNowHint: "Opens the full-screen overlay: drag to select, then annotate, copy or save. Esc leaves it.",
  tryNowAction: "Capture",
};
