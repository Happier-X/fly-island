export type AgentText = {
  running: (provider: string) => string;
  failed: (provider: string) => string;
  stale: (provider: string) => string;
  idleOrDone: string;
  phase: {
    running: string;
    completed: string;
    failed: string;
    stale: string;
    waiting: string;
    idle: string;
  };
  sessions: {
    title: string;
    open: string;
    empty: string;
    emptyHint: string;
    notFound: string;
    notFoundEnded: string;
    justNow: string;
    minutesAgo: (minutes: number) => string;
    activeCount: (count: number) => string;
    duration: (parts: { hours: number; minutes: number; seconds: number }) => string;
    elapsedRunning: (duration: string) => string;
    elapsedDone: (duration: string) => string;
    /** Badges for the modes in which a session acts without asking; the default mode has none. */
    permissionModes: Record<"auto" | "bypass" | "acceptEdits" | "plan", string>;
    /** The context size a card shows at its right end (formatTokens). */
    contextTokens: (size: string) => string;
    remove: (title: string) => string;
    /** Heading of sessions without a known folder, when grouped by project. */
    noProject: string;
    /** The Agent page's arrange menu in the island header: the order and the headings. */
    arrange: {
      label: string;
      sort: string;
      group: string;
      sortOptions: { status: string; recent: string; firstSeen: string };
      groupOptions: { project: string; none: string };
    };
    removed: string;
    undo: string;
  };
};

export const zhCN: AgentText = {
  running: (provider) => `${provider} 正在运行`,
  failed: (provider) => `${provider} 运行失败`,
  stale: (provider) => `${provider} 可能已中断`,
  idleOrDone: "AI Agent 空闲或已完成",
  phase: {
    running: "正在运行",
    completed: "已完成",
    failed: "运行失败",
    stale: "可能已中断",
    waiting: "等待确认",
    idle: "空闲",
  },
  sessions: {
    title: "Agent 会话",
    open: "查看 Agent 会话",
    empty: "最近 2 小时没有 Agent 会话",
    emptyHint: "在 Cindy 里开始任务后会显示在这里",
    notFound: "没找到 Cindy 的窗口",
    notFoundEnded: "没找到 Cindy 的窗口，它可能已经关掉了",
    justNow: "刚刚",
    minutesAgo: (minutes) => (minutes < 60 ? `${minutes} 分` : `${Math.floor(minutes / 60)} 小时`),
    activeCount: (count) => `${count} 个 Agent 会话正在运行，点击查看`,
    duration: ({ hours, minutes, seconds }) =>
      hours > 0 ? `${hours} 小时 ${minutes} 分` : minutes > 0 ? `${minutes} 分 ${seconds} 秒` : `${seconds} 秒`,
    elapsedRunning: (duration) => `已运行 ${duration}`,
    elapsedDone: (duration) => `用时 ${duration}`,
    permissionModes: { auto: "自动", bypass: "跳过确认", acceptEdits: "自动改文件", plan: "计划" },
    contextTokens: (size) => `${size} tokens`,
    remove: (title) => `归档「${title}」`,
    noProject: "未知项目",
    arrange: {
      label: "排列会话",
      sort: "排序",
      group: "分组",
      sortOptions: { status: "状态优先", recent: "最近活动", firstSeen: "首次出现" },
      groupOptions: { project: "按项目", none: "不分组" },
    },
    removed: "已归档",
    undo: "撤销",
  },
};

export const en: AgentText = {
  running: (provider) => `${provider} is running`,
  failed: (provider) => `${provider} failed`,
  stale: (provider) => `${provider} may have stalled`,
  idleOrDone: "AI Agent idle or finished",
  phase: {
    running: "Running",
    completed: "Done",
    failed: "Failed",
    stale: "Stalled",
    waiting: "Waiting for you",
    idle: "Idle",
  },
  sessions: {
    title: "Agent sessions",
    open: "Show agent sessions",
    empty: "No agent sessions in the last 2 hours",
    emptyHint: "Tasks you start in Cindy show up here",
    notFound: "No Cindy window found",
    notFoundEnded: "No Cindy window; it may have been closed",
    justNow: "<1m",
    minutesAgo: (minutes) => (minutes < 60 ? `${minutes}m` : `${Math.floor(minutes / 60)}h`),
    activeCount: (count) => (count === 1 ? "1 agent session running; click to view" : `${count} agent sessions running; click to view`),
    duration: ({ hours, minutes, seconds }) =>
      hours > 0 ? `${hours}h ${minutes}m` : minutes > 0 ? `${minutes}m ${seconds}s` : `${seconds}s`,
    elapsedRunning: (duration) => `running ${duration}`,
    elapsedDone: (duration) => `took ${duration}`,
    permissionModes: { auto: "Auto", bypass: "Bypass", acceptEdits: "Accept edits", plan: "Plan" },
    contextTokens: (size) => `${size} tokens`,
    remove: (title) => `Archive "${title}"`,
    noProject: "No project",
    arrange: {
      label: "Arrange sessions",
      sort: "Sort",
      group: "Group",
      sortOptions: { status: "Status", recent: "Recent", firstSeen: "First seen" },
      groupOptions: { project: "Project", none: "None" },
    },
    removed: "Archived",
    undo: "Undo",
  },
};
