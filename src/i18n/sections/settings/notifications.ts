export type SettingsNotificationsText = {
  enable: string;
  events: {
    title: string;
    hint: string;
    done: string;
    question: string;
    scheduled: string;
    error: string;
  };
  sound: {
    title: string;
    enabled: string;
    hint: string;
  };
  source: {
    title: string;
    hint: string;
    /** How many events this read found, and when it read them. */
    reading: (count: number, time: string) => string;
    /** Cindy is not installed, or has not created its database yet. */
    missing: string;
    /** The database is there but could not be read (locked, or Cindy changed its tables). */
    failed: string;
    refresh: string;
  };
};

export const zhCN: SettingsNotificationsText = {
  enable: "启用 Cindy 通知",
  events: {
    title: "提醒哪些事",
    hint: "有未处理的通知时，收起的岛右侧显示数量；点一下把 Cindy 的窗口拿到前台。Cindy 已在前台时不再提醒。",
    done: "任务完成",
    question: "需要你回答",
    scheduled: "定时与后台任务",
    error: "出错",
  },
  sound: {
    title: "提示音",
    enabled: "响提示音",
    hint: "有新通知时播放提示音；一次来好几条只响最重要的一条。",
  },
  source: {
    title: "数据来源",
    hint: "直接读 Cindy 自己的本地数据库，不装钩子、不改 Cindy 的任何设置。Cindy 没开时读到的就是空的。",
    reading: (count, time) => `最近读取 ${time}，读到 ${count} 条通知`,
    missing: "没找到 Cindy 的数据库（Cindy 没装，或还没建过会话）",
    failed: "数据库读不出来，可能被 Cindy 独占，或它改了表结构",
    refresh: "重新读取",
  },
};

export const en: SettingsNotificationsText = {
  enable: "Enable Cindy notifications",
  events: {
    title: "What to tell you about",
    hint: "While notifications are unhandled, the collapsed island shows how many at its right; clicking brings Cindy forward. Nothing is shown while Cindy is in front.",
    done: "A task finished",
    question: "It needs your answer",
    scheduled: "Scheduled and background tasks",
    error: "Something went wrong",
  },
  sound: {
    title: "Sound",
    enabled: "Play a sound",
    hint: "Plays a sound for new notifications; several at once play only the most important one.",
  },
  source: {
    title: "Data source",
    hint: "Read straight from Cindy's own local database: no hooks, and nothing in Cindy changes. While Cindy is closed, it reads as empty.",
    reading: (count, time) => `Read at ${time}: ${count} notifications`,
    missing: "Cindy's database was not found (Cindy is not installed, or has had no sessions yet)",
    failed: "The database could not be read: Cindy may hold it, or has changed its tables",
    refresh: "Read again",
  },
};
