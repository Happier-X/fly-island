export type SettingsCategoriesText = {
  label: string;
  groups: {
    basics: string;
    features: string;
  };
  general: string;
  appearance: string;
  shortcuts: string;
  /** QQ, WeChat and Mail on one page. */
  messages: string;
  /** Mail's group on the Messages page. */
  mail: string;
  todos: string;
  music: string;
  clipboard: string;
  screenshot: string;
  translation: string;
  /** The file shelf's group on the Clipboard page. */
  shelf: string;
  /** Cindy notifications: one page with the switches, the sound and the data source. */
  notifications: string;
  about: string;
};

export const zhCN: SettingsCategoriesText = {
  label: "设置分类",
  groups: {
    basics: "基础",
    features: "功能",
  },
  general: "通用",
  appearance: "外观",
  shortcuts: "快捷键",
  messages: "消息",
  mail: "邮箱",
  todos: "待办",
  music: "音乐",
  clipboard: "剪贴板与文件",
  screenshot: "截图",
  translation: "翻译",
  shelf: "文件中转",
  notifications: "Cindy 通知",
  about: "关于",
};

export const en: SettingsCategoriesText = {
  label: "Settings categories",
  groups: {
    basics: "Basics",
    features: "Features",
  },
  general: "General",
  appearance: "Appearance",
  shortcuts: "Shortcuts",
  messages: "Messages",
  mail: "Mail",
  todos: "Todos",
  music: "Music",
  clipboard: "Clipboard & files",
  screenshot: "Screenshots",
  translation: "Translation",
  shelf: "File shelf",
  notifications: "Cindy notifications",
  about: "About",
};
