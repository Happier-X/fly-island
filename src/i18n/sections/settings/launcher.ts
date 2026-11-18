export type SettingsLauncherText = {
  enable: string;
  enableHint: string;
  shortcut: string;
  pressKeys: string;
  searchScope: string;
  apps: string;
  files: string;
  web: string;
  engine: string;
  engineHint: string;
  engines: Record<"bing" | "google" | "baidu" | "duckduckgo", string>;
  folders: string;
  foldersHint: string;
  foldersEmpty: string;
  addFolder: string;
  folderPlaceholder: string;
  removeFolder: (path: string) => string;
  useDefaultFolders: string;
  shortcutHint: string;
};

export const zhCN: SettingsLauncherText = {
  enable: "启用快捷启动",
  enableHint: "关闭后释放全局快捷键，面板不再弹出；设置会保留。",
  shortcut: "打开快捷启动",
  pressKeys: "按下快捷键",
  searchScope: "搜索范围",
  apps: "搜索应用",
  files: "搜索文件",
  web: "无匹配时搜索网页",
  engine: "默认搜索引擎",
  engineHint: "回车用默认浏览器打开搜索结果。",
  engines: {
    bing: "必应",
    google: "Google",
    baidu: "百度",
    duckduckgo: "DuckDuckGo",
  },
  folders: "文件搜索目录",
  foldersHint: "搜索桌面、文档和下载中的文件，最多向下 3 层、最多 3000 个文件；可改用其他目录，不存在的目录会被忽略。",
  foldersEmpty: "还没有添加目录。",
  addFolder: "添加",
  folderPlaceholder: "输入文件夹路径，例如 D:\\资料",
  removeFolder: (path) => `移除 ${path}`,
  useDefaultFolders: "恢复默认目录",
  shortcutHint: "在任何程序里按这个组合键都会弹出快捷启动。",
};

export const en: SettingsLauncherText = {
  enable: "Enable quick launcher",
  enableHint: "When off, the shortcut is released and the panel stays closed. Your settings are kept.",
  shortcut: "Open quick launcher",
  pressKeys: "Press a shortcut",
  searchScope: "Search scope",
  apps: "Search apps",
  files: "Search files",
  web: "Search the web when nothing matches",
  engine: "Default search engine",
  engineHint: "Enter opens the web result in your default browser.",
  engines: {
    bing: "Bing",
    google: "Google",
    baidu: "Baidu",
    duckduckgo: "DuckDuckGo",
  },
  folders: "File search folders",
  foldersHint: "Searches Desktop, Documents and Downloads up to 3 folders deep, at most 3000 files; other folders can be used instead, and missing ones are ignored.",
  foldersEmpty: "No folders yet.",
  addFolder: "Add",
  folderPlaceholder: "Type a folder path, for example D:\\Files",
  removeFolder: (path) => `Remove ${path}`,
  useDefaultFolders: "Restore the default folders",
  shortcutHint: "The panel opens in any program when this combination is pressed.",
};
