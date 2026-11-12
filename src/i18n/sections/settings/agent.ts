export type SettingsAgentText = {
  title: string;
  source: {
    title: string;
    hint: string;
    path: string;
    /** The database is there and the sessions read fine. */
    reading: (count: number) => string;
    /** Cindy is not installed, or has not created its database yet. */
    missing: string;
    /** The database is there but could not be read (locked, or Cindy changed its tables). */
    failed: string;
    refresh: string;
  };
  sessionTitleSection: string;
  sessionTitle: string;
  sessionTitleHint: string;
  alerts: {
    title: string;
    hint: string;
    completed: string;
    waiting: string;
  };
};

export const zhCN: SettingsAgentText = {
  title: "AI Agent 状态灯",
  source: {
    title: "数据来源",
    hint: "直接读 Cindy 自己的会话数据库，不装钩子、不改 Cindy 的任何设置。Cindy 没开时读到的就是空的。",
    path: "数据库",
    reading: (count) => `已读取 ${count} 个会话`,
    missing: "没找到 Cindy 的数据库（Cindy 没装，或还没建过会话）",
    failed: "数据库读不出来，可能被 Cindy 独占，或它改了表结构",
    refresh: "重新读取",
  },
  sessionTitleSection: "小岛显示",
  sessionTitle: "运行时在小岛显示会话标题",
  sessionTitleHint: "显示 Cindy 的图标和会话标题，结束 3 秒后恢复待办。",
  alerts: {
    title: "弹出提醒",
    hint: "收起的岛屿会自动打开 Agent 页。Cindy 已在前台、游戏模式勿扰时，或你正在用展开的岛屿时不弹。",
    completed: "会话完成时",
    waiting: "等你回答时",
  },
};

export const en: SettingsAgentText = {
  title: "AI Agent status light",
  source: {
    title: "Data source",
    hint: "Read straight from Cindy's own session database: no hooks, and nothing in Cindy changes. While Cindy is closed, it reads as empty.",
    path: "Database",
    reading: (count) => (count === 1 ? "1 session read" : `${count} sessions read`),
    missing: "Cindy's database was not found (Cindy is not installed, or has had no sessions yet)",
    failed: "The database could not be read: Cindy may hold it, or has changed its tables",
    refresh: "Read again",
  },
  sessionTitleSection: "On the island",
  sessionTitle: "Show the session title while running",
  sessionTitleHint: "Shows the Cindy icon and session title, then the todo again 3 seconds after it finishes.",
  alerts: {
    title: "Pop-up alerts",
    hint: "A collapsed island opens the Agent page by itself. Not while Cindy is in front, game mode keeps the island quiet, or you are using the open island.",
    completed: "When a session finishes",
    waiting: "When it waits for your answer",
  },
};
