export type SettingsAppearanceText = {
  font: string;
  fontLabel: string;
  systemFont: string;
  pixelFont: string;
  title: string;
  classic: string;
  liquidGlass: string;
  glassStrength: string;
  styleTitle: string;
  /** The island preview at the top of the page: sample content and the two captions. */
  preview: {
    label: string;
    collapsed: string;
    expanded: string;
    /** Sample content in the drawings: the collapsed bar's text and three todos. */
    collapsedText: string;
    todos: { one: string; two: string; three: string };
  };
  /** The "Collapsed island" group: what its text shows first, and the song's cover. */
  collapsedIsland: {
    title: string;
    priorityFooter: string;
    items: Record<"lyrics" | "todo", { name: string; hint: string }>;
    /** The drag handle of one item; arrow keys on it move the item too. */
    moveLabel: (name: string) => string;
    showCover: string;
    showCoverHint: string;
    layout: {
      title: string;
      footer: string;
      reset: string;
      slots: Record<"text" | "mail" | "shelf" | "music" | "notifications", string>;
      textSample: string;
      moveLabel: (name: string) => string;
    };
  };
};

export const zhCN: SettingsAppearanceText = {
  font: "字体",
  fontLabel: "岛屿字体",
  systemFont: "默认字体",
  pixelFont: "像素字体",
  title: "外观模式",
  classic: "经典",
  liquidGlass: "液态玻璃",
  glassStrength: "玻璃强度",
  styleTitle: "风格",
  preview: {
    label: "按当前设置画出的岛屿",
    collapsed: "折叠",
    expanded: "展开",
    collapsedText: "剩余 2 个待办",
    todos: { one: "整理本周的发布说明", two: "修复设置页的滚动跳动", three: "回访邀请测试的用户" },
  },
  collapsedIsland: {
    title: "收起的岛",
    priorityFooter: "几样同时有内容时，收起的岛显示排在最前面的那一样。拖动调整顺序。",
    items: {
      lyrics: { name: "歌词", hint: "播放时的当前一句，没有歌词时显示歌名" },
      todo: { name: "待办", hint: "开始做的待办；没开始时，剩余数量排在最后" },
    },
    moveLabel: (name) => `调整"${name}"的顺序，可以拖动或按上下方向键`,
    showCover: "显示歌曲封面",
    showCoverHint: "收起的岛显示歌词时，在歌词前显示这首歌的封面；排在歌词前面的内容显示时，封面也一起让开。",
    layout: {
      title: "岛上的排列",
      footer: "按住图标左右拖动，放在文字左边的会显示在岛的左侧。灰色的是还没打开的功能，位置会保留。",
      reset: "恢复默认",
      slots: { text: "文字", mail: "新邮件", shelf: "置物架文件数", music: "音乐波形", notifications: "Cindy 通知" },
      textSample: "收起时的文字",
      moveLabel: (name) => `移动"${name}"，可以拖动或按左右方向键`,
    },
  },
};

export const en: SettingsAppearanceText = {
  font: "Font",
  fontLabel: "Island font",
  systemFont: "System",
  pixelFont: "Pixel",
  title: "Appearance",
  classic: "Classic",
  liquidGlass: "Liquid glass",
  glassStrength: "Glass strength",
  styleTitle: "Style",
  preview: {
    label: "The island with the current settings",
    collapsed: "Collapsed",
    expanded: "Expanded",
    collapsedText: "2 todos left",
    todos: { one: "Draft this week's release notes", two: "Fix the settings page scroll jump", three: "Follow up with beta users" },
  },
  collapsedIsland: {
    title: "Collapsed island",
    priorityFooter: "When several have something to show, the collapsed island shows the first. Drag to reorder.",
    items: {
      lyrics: { name: "Lyrics", hint: "The current line while a song plays, or its title" },
      todo: { name: "Todos", hint: "The todo you started; until then, the count left comes last" },
    },
    moveLabel: (name) => `Reorder "${name}": drag, or press the up and down arrow keys`,
    showCover: "Show the song's cover",
    showCoverHint: "Beside the lyric line on the collapsed island; when something ranked above lyrics shows, the cover gives way too.",
    layout: {
      title: "On the island",
      footer: "Hold an icon and drag it left or right; icons left of the text sit on the island's left. Grey ones are switched off and keep their place.",
      reset: "Reset",
      slots: { text: "Text", mail: "New mail", shelf: "Shelf files", music: "Music waveform", notifications: "Cindy notifications" },
      textSample: "Collapsed text",
      moveLabel: (name) => `Move "${name}": drag, or press the left and right arrow keys`,
    },
  },
};
