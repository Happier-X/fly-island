export type LauncherText = {
  title: string;
  placeholder: string;
  emptyHint: string;
  noResults: (query: string) => string;
  searchWeb: (query: string) => string;
  results: string;
  loading: string;
  openFailed: (name: string) => string;
  indexFailed: string;
  hintSelect: string;
  hintOpen: string;
  hintClose: string;
};

export const zhCN: LauncherText = {
  title: "快捷启动",
  placeholder: "搜索应用、文件或网页",
  emptyHint: "输入名称，搜索应用、文件，或按回车搜索网页。",
  noResults: (query) => `没有找到与“${query}”匹配的应用或文件。`,
  searchWeb: (query) => `在网页中搜索“${query}”`,
  results: "搜索结果",
  loading: "正在加载应用与文件…",
  openFailed: (name) => `无法打开“${name}”，请检查它是否仍然存在。`,
  indexFailed: "读取搜索索引失败，请稍后再试。",
  hintSelect: "选择",
  hintOpen: "打开",
  hintClose: "关闭",
};

export const en: LauncherText = {
  title: "Quick launcher",
  placeholder: "Search apps, files or the web",
  emptyHint: "Type a name to search apps and files, or press Enter to search the web.",
  noResults: (query) => `No app or file matches “${query}”.`,
  searchWeb: (query) => `Search the web for “${query}”`,
  results: "Search results",
  loading: "Loading apps and files…",
  openFailed: (name) => `Could not open “${name}”. Check that it still exists.`,
  indexFailed: "Could not read the search index. Try again in a moment.",
  hintSelect: "select",
  hintOpen: "open",
  hintClose: "close",
};
