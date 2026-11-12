// pnpm dev, then open /tests/regression/agent-sessions.html?test=1 (without ?test=1 the page is for
// visual QA; add &empty=1 to see the empty state, &rows=N to size it for N rows like the island does,
// &view=cards or &view=compact for the other layouts, &effects=sparkle or gradient on them).
// The sessions are Cindy's: every one runs in Cindy's own window, so a click only brings that window
// forward.
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { mockIPC } from "@tauri-apps/api/mocks";
import "../../src/styles/tokens.css";
import { I18nProvider } from "../../src/i18n";
import { AgentSessionsPanel } from "../../src/features/agent-status/AgentSessionsPanel";
import type { AgentCardEffects, AgentSessionView } from "../../src/features/appearance/types";
import type { AgentSessionSummary } from "../../src/features/agent-status/api";
import { getAgentPageExpandedHeight } from "../../src/windows/island/layout";

// Island padding, header and gap above the page content; see AGENT_PAGE_CHROME_HEIGHT in layout.ts.
const ISLAND_CHROME_HEIGHT = 86;
const fixture = document.getElementById("fixture")!;
const sizeFor = (rows: number) => { fixture.style.height = `${getAgentPageExpandedHeight(rows) - ISLAND_CHROME_HEIGHT}px`; };

Object.assign(globalThis, { isTauri: true, IS_REACT_ACT_ENVIRONMENT: true });
const params = new URLSearchParams(location.search);
const now = Date.now();
let focusCalls = 0;
// Cindy is running unless a check says otherwise: then the click finds no window.
let cindyRunning = true;
mockIPC((command) => {
  if (command === "focus_agent_window") {
    focusCalls += 1;
    return cindyRunning;
  }
});

const SESSIONS: AgentSessionSummary[] = [
  {
    provider: "cindy", sessionId: "a", cwd: "E:\\Projects\\fly-island", title: "设置界面完善思路",
    model: "Opus 5.5", effort: null, phase: "running", updatedAt: now - 20_000, turnStartedAt: now - 200_000,
    permissionMode: "bypass", contextTokens: 86_400, contextWindow: 200_000,
    // As long as the native side sends (120 characters), so it needs more than two lines.
    lastMessage: "24 个测试全部通过。接下来用真实数据看看列表的实际效果和耗时，再决定要不要给第一句提问加缓存，顺便检查一下项目名是不是会话开始时的目录，以及跳转窗口时能不能按项目文件夹找到对应的编辑器窗口和终端窗口…",
  },
  {
    provider: "cindy", sessionId: "b", cwd: "E:\\Projects\\MioPicFlow", title: "恢复 Windows 发布机环境，顺便把代理配置也整理一遍",
    model: "gpt-6-astra", effort: null, phase: "waiting", updatedAt: now - 14 * 60_000, turnStartedAt: null, permissionMode: "default",
    lastMessage: "关闭系统代理后国内网站立即恢复，说明问题集中在 Clash 的代理链路。",
  },
  {
    provider: "cindy", sessionId: "c", cwd: "E:\\Projects\\TixXinBlog", title: "Nexus 紧凑视口密度变量",
    model: "Opus 5.5", effort: null, phase: "completed", permissionMode: "auto", updatedAt: now - 95 * 60_000, lastMessage: null, turnStartedAt: now - 95 * 60_000 - 252_000,
  },
];

fixture.style.height = `${getAgentPageExpandedHeight(Number(params.get("rows")) || 3, (params.get("view") as AgentSessionView | null) ?? "list") - ISLAND_CHROME_HEIGHT}px`;
const root = createRoot(fixture);
const sessions = params.has("empty") ? [] : SESSIONS;
const render = async (cards: { showElapsed?: boolean; showModel?: boolean } = {}) => {
  await act(async () => {
    root.render(<StrictMode><I18nProvider><AgentSessionsPanel sort="recent" sessions={sessions} view={(params.get("view") as AgentSessionView | null) ?? "list"} cardEffects={(params.get("effects") as AgentCardEffects | null) ?? "off"} {...cards} /></I18nProvider></StrictMode>);
    await new Promise((resolve) => setTimeout(resolve, 20));
  });
};
await render();

const messages: string[] = [];
function check(condition: unknown, label: string) {
  if (!condition) throw new Error(label);
  messages.push(`PASS ${label}`);
}
const rows = () => [...document.querySelectorAll<HTMLButtonElement>(".agent-session")];
const results = document.getElementById("results")!;

async function runChecks() {
  try {
    check(rows().length === 3, "列出全部会话");
    const [first, second, third] = rows();
    check(first.textContent?.includes("fly-island · 设置界面完善思路") && first.querySelector(".agent-provider-icon--cindy"), "每行显示 Cindy 图标、项目名和标题");
    check(first.querySelector(".agent-session__model")?.textContent === "Opus 5.5" && second.querySelector(".agent-session__model")?.textContent === "gpt-6-astra", "显示 Cindy 记录的模型名");
    check(first.classList.contains("agent-session--running") && second.classList.contains("agent-session--waiting") && third.classList.contains("agent-session--completed"), "状态灯按会话状态着色");
    const meta = (row: HTMLElement) => row.querySelector(".agent-session__meta")?.textContent;
    const dot = (row: HTMLElement) => getComputedStyle(row.querySelector(".agent-session__dot")!).backgroundColor;
    check(meta(second) === "等待确认 · 14 分" && dot(second) === "rgb(255, 69, 58)", "在等你回答的会话显示红色状态灯和状态文字");
    check(dot(first) === "rgb(52, 199, 89)" && dot(third) === "rgb(142, 142, 147)", "在跑的会话绿色，做完的灰色");
    check(meta(first)?.startsWith("已运行 3 分 2") && !meta(first)?.includes("刚刚") && meta(third)?.endsWith("1 小时"), "运行中的会话显示已运行多久，结束的显示多久前更新");
    check(first.classList.contains("agent-session--full") && second.classList.contains("agent-session--full") && third.classList.contains("agent-session--compact"), "进行中的会话展开，已完成的折叠成一行");
    check(rows().every((row) => row.firstElementChild?.classList.contains("agent-session__dot") && row.querySelector(".agent-session__heading .agent-provider-icon") && row.querySelectorAll(".agent-session__dot").length === 1), "每行都以状态灯开头，图标都在标题行右侧");
    const title = second.querySelector<HTMLElement>(".agent-session__title")!;
    const message = first.querySelector<HTMLElement>(".agent-session__message")!;
    check(title.scrollWidth > title.clientWidth, "长标题单行省略");
    check(message.scrollHeight > message.clientHeight && message.clientHeight < 40, "最新回复最多两行");
    check(!third.querySelector(".agent-session__message") && !third.querySelector(".agent-session__model"), "折叠行不显示回复和模型");
    check(first.querySelector(".agent-session__mode--bypass")?.textContent === "跳过确认", "运行中的会话标出跳过确认的权限模式");
    check(!second.querySelector(".agent-session__mode") && !third.querySelector(".agent-session__mode"), "默认权限模式和已结束的会话不显示模式");
    const titleTone = (row: HTMLElement) => getComputedStyle(row.querySelector(".agent-session__title")!).color;
    const dimTitle = titleTone(third);
    const seenKeys: string[] = [];
    await act(async () => {
      root.render(<StrictMode><I18nProvider><AgentSessionsPanel sort="recent" sessions={sessions} unseen={{ keys: new Set(["cindy:c"]), markSeen: (session) => seenKeys.push(session.sessionId) }} /></I18nProvider></StrictMode>);
      await new Promise((resolve) => setTimeout(resolve, 20));
    });
    check(third.classList.contains("agent-session--unseen") && titleTone(third) === titleTone(first) && titleTone(third) !== dimTitle && !first.classList.contains("agent-session--unseen"), "跑完还没看的会话标题保持亮色，看过的才变灰");
    first.click();
    check(seenKeys.join() === "a", "点开会话就算看过");
    const removes: string[] = [];
    let undone = 0;
    const renderRemoval = async (removed: AgentSessionSummary | null) => {
      await act(async () => {
        root.render(<StrictMode><I18nProvider><AgentSessionsPanel sort="recent" sessions={sessions}
          removal={{ remove: (session) => removes.push(session.sessionId), removed, undo: () => { undone += 1; } }} /></I18nProvider></StrictMode>);
        await new Promise((resolve) => setTimeout(resolve, 20));
      });
    };
    await renderRemoval(null);
    const removeButtons = [...document.querySelectorAll<HTMLButtonElement>(".agent-session__archive")];
    const firstRow = rows()[0].getBoundingClientRect();
    const firstRemove = removeButtons[0].getBoundingClientRect();
    const providerIcon = rows()[0].querySelector(".agent-session__heading > :last-child")!.getBoundingClientRect();
    const archiveIcon = removeButtons[0].querySelector("svg")!.getBoundingClientRect();
    check(removeButtons.length === 3 && getComputedStyle(removeButtons[0]).opacity === "1"
      && firstRemove.left >= providerIcon.right && firstRemove.right <= firstRow.right && firstRemove.width <= 20
      && Math.abs(fixture.getBoundingClientRect().right + 20 - archiveIcon.right - 20) < 1,
      "归档按钮一直显示在 Cindy 图标右边，图标离岛屿边缘 20px，和顶部按钮对齐");
    check(removeButtons[0].getAttribute("aria-label") === "归档「设置界面完善思路」", "归档按钮的读屏文字说明归档哪个会话");
    const focusesBefore = focusCalls;
    removeButtons[2].click();
    check(removes.length === 0, "点归档后，行先播完移出动画才离开列表");
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 400)); });
    check(removes.join() === "c" && focusCalls === focusesBefore, "动画播完只归档这个会话，不去切换窗口");
    await renderRemoval(sessions[2]);
    const notice = document.querySelector<HTMLElement>(".agent-archived");
    const panelBox = document.querySelector<HTMLElement>(".agent-sessions")!.getBoundingClientRect();
    check(notice?.textContent?.includes("已归档") && notice.getAttribute("role") === "status"
      && getComputedStyle(notice).position === "absolute" && notice.getBoundingClientRect().left - panelBox.left < 12
      && notice.getBoundingClientRect().width < panelBox.width / 2,
      "归档后左下角浮出「已归档 · 撤销」小胶囊，不占列表的位置");
    notice!.querySelector("button")!.click();
    check(undone === 1, "点撤销把会话放回去");
    await renderRemoval(null);
    check(document.querySelector(".agent-archived--leaving"), "撤销或到时后胶囊先播移出动画");
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 400)); });
    check(!document.querySelector(".agent-archived"), "移出动画播完胶囊消失");
    await render();
    const panel = document.querySelector<HTMLElement>(".agent-sessions")!;
    check(panel.scrollWidth <= panel.clientWidth, "面板没有横向溢出");
    const list = document.querySelector<HTMLElement>(".agent-sessions__list")!;
    const rowHeights = rows().map((row) => row.getBoundingClientRect().height);
    check(rowHeights[0] === 78 && rowHeights[1] < rowHeights[0] - 10 && rowHeights[2] < rowHeights[1], "展开行随回复行数变高（两行 78px，一行更矮，不留空行），折叠行最矮");
    const font = (selector: string) => { const style = getComputedStyle(first.querySelector(selector)!); return `${style.fontSize} ${style.fontWeight}`; };
    check(font(".agent-session__title") === "14px 600" && font(".agent-session__message") === "13px 400" && font(".agent-session__meta") === "12px 400" && font(".agent-session__model") === "11px 400", "字号字重：标题 14/600，回复 13/400，状态 12/400，模型 11/400");
    check(list.scrollHeight <= list.clientHeight, "设为 3 条时放下 3 个会话，不滚动");
    check(Math.abs(list.getBoundingClientRect().bottom - (fixture.getBoundingClientRect().bottom + 20)) < 1, "列表一直延伸到岛屿底边，底部不留空白条");
    const afterLayout = () => act(async () => { await new Promise((resolve) => setTimeout(resolve, 50)); });
    const masked = () => getComputedStyle(list).maskImage !== "none";
    check(!list.dataset.fadeTop && !list.dataset.fadeBottom && !masked(), "放得下时两头都不渐隐");
    sizeFor(2);
    await afterLayout();
    check(list.scrollHeight > list.clientHeight, "设为 2 条时第 3 个会话需要滚动");
    check(list.dataset.fadeBottom !== undefined && list.dataset.fadeTop === undefined && masked(), "下面还有会话时底部渐隐，顶部不渐隐");
    list.scrollTop = list.scrollHeight;
    await afterLayout();
    check(list.dataset.fadeTop !== undefined && list.dataset.fadeBottom === undefined, "滚到底后顶部渐隐，底部不再渐隐");
    list.scrollTop = 0;
    sizeFor(5);
    await afterLayout();
    check(list.scrollHeight <= list.clientHeight, "设为 5 条时 3 个会话不滚动");
    sizeFor(3);

    // The fixture is the island's content box; the island's edge is its 20px side padding further out.
    await render();
    const iconRight = (row: HTMLElement) => row.querySelector(".agent-provider-icon")!.getBoundingClientRect().right;
    const dotGap = first.querySelector(".agent-session__dot")!.getBoundingClientRect().left - (fixture.getBoundingClientRect().left - 20);
    const iconGap = fixture.getBoundingClientRect().right + 20 - iconRight(first);
    check(Math.abs(dotGap - 20) < 1 && Math.abs(iconGap - 20) < 1 && Math.abs(iconRight(first) - iconRight(third)) < 0.5, "圆点和右侧图标都距岛屿边缘 20px，图标在同一列");
    check(meta(third) === "用时 4 分 12 秒 · 1 小时", "结束的会话显示这一轮用时");
    check(!second.querySelector(".agent-session__elapsed"), "不知道这一轮何时开始时不显示时长");
    const before = first.querySelector(".agent-session__elapsed")?.textContent;
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 1100)); });
    check(first.querySelector(".agent-session__elapsed")?.textContent !== before, "进行中的会话时长每秒更新");
    await render({ showElapsed: false, showModel: false });
    check(!document.querySelector(".agent-session__elapsed") && !document.querySelector(".agent-session__model"), "可以关掉运行时长和模型");
    await act(async () => {
      root.render(<StrictMode><I18nProvider><AgentSessionsPanel sort="recent" sessions={[{ ...SESSIONS[0], lastMessage: null }, ...SESSIONS.slice(1)]} /></I18nProvider></StrictMode>);
    });
    check(rows()[0].classList.contains("agent-session--compact") && rows()[0].querySelector(".agent-session__model") && rows()[0].getBoundingClientRect().height < 40, "刚开始一轮、还没有回复的会话不留空白第二行");
    await render();
    const titleColor = () => getComputedStyle(rows()[0].querySelector(".agent-session__title")!).color;
    check(titleColor() === "rgba(241, 240, 246, 0.96)", "深色岛屿背景上标题是浅色字");
    fixture.style.setProperty("--island-ink-rgb", "24, 24, 30");
    check(titleColor() === "rgba(24, 24, 30, 0.96)", "浅色岛屿背景上标题换成深色字");
    fixture.style.removeProperty("--island-ink-rgb");

    const focusesBeforeClick = focusCalls;
    await act(async () => { first.click(); await Promise.resolve(); });
    check(focusCalls === focusesBeforeClick + 1, "点一个会话就把 Cindy 拿到前面");
    check(!document.querySelector(".agent-session__not-found"), "找到 Cindy 的窗口时不提示");
    cindyRunning = false;
    await act(async () => { second.click(); await new Promise((resolve) => setTimeout(resolve, 20)); });
    check(second.querySelector(".agent-session__not-found")?.textContent === "没找到 Cindy 的窗口", "Cindy 关着时在该行提示");
    await act(async () => { third.click(); await new Promise((resolve) => setTimeout(resolve, 20)); });
    check(third.querySelector(".agent-session__not-found")?.textContent === "没找到 Cindy 的窗口，它可能已经关掉了", "已结束的会话找不到窗口时说明可能已关掉");
    cindyRunning = true;

    // The card and compact layouts (Settings), each sized by its own row height.
    const renderView = async (view: AgentSessionView) => {
      fixture.style.height = `${getAgentPageExpandedHeight(3, view) - ISLAND_CHROME_HEIGHT}px`;
      await act(async () => {
        root.render(<StrictMode><I18nProvider><AgentSessionsPanel sort="recent" sessions={sessions} view={view}
          removal={{ remove: () => undefined, removed: null, undo: () => undefined }} /></I18nProvider></StrictMode>);
        await new Promise((resolve) => setTimeout(resolve, 20));
      });
      return [...document.querySelectorAll<HTMLButtonElement>(".agent-card")];
    };
    const fits = () => { const box = document.querySelector<HTMLElement>(".agent-sessions__list")!; return box.scrollHeight <= box.clientHeight + 1; };
    let cards = await renderView("cards");
    check(cards.length === 3 && cards.every((card) => card.offsetHeight === 62) && fits(), "卡片视图每个会话一张 62px 的卡片，三张正好放下");
    check(cards[0].querySelector(".agent-card__tokens")?.textContent === "86.4k tokens" && !cards[2].querySelector(".agent-card__tokens"), "卡片右下写上下文大小，不知道时不写");
    check(cards.every((card) => card.parentElement?.querySelector(".agent-session__archive")), "卡片视图照样能归档");
    cards = await renderView("compact");
    check(cards.length === 3 && cards.every((card) => card.offsetHeight === 34 && !card.querySelector(".agent-card__status-line") && card.querySelector(".agent-card__bar")) && fits(),
      "紧凑视图每个会话一行 34px，只有色条、标题、状态和时间");
    const dissolved: string[] = [];
    await act(async () => root.render(<StrictMode><I18nProvider><AgentSessionsPanel sort="recent" sessions={sessions} view="cards" archiveMotion="dissolve"
      removal={{ remove: (session) => dissolved.push(session.sessionId), removed: null, undo: () => undefined }} /></I18nProvider></StrictMode>));
    await act(async () => document.querySelector<HTMLButtonElement>(".agent-session__archive")!.click());
    check(document.querySelector(".agent-dissolving > .agent-dissolve__dust") && dissolved.length === 0, "选了粒子消散时，归档的卡片先化成粒子，还没从列表移除");
    await act(async () => { await new Promise((resolve) => setTimeout(resolve, 800)); });
    check(dissolved.join() === "a", "粒子散完、收起后才移除这个会话");
    await act(async () => root.render(<StrictMode><I18nProvider><AgentSessionsPanel sort="recent" sessions={sessions} view="cards" cardEffects="sparkle" /></I18nProvider></StrictMode>));
    check(document.querySelectorAll(".agent-card--sparkle > .agent-card__band").length === 3
      && [...document.querySelectorAll<HTMLElement>(".agent-card--sparkle")].every((card) => card.offsetHeight === 62), "粒子流苏给每张卡片加一条流苏，不改变卡片高度");
    results.textContent = `${messages.join("\n")}\n\n全部 ${messages.length} 项检查通过。`;
    results.dataset.result = "passed";
  } catch (error) {
    results.textContent = `${messages.join("\n")}\nFAIL ${String(error)}`;
    results.dataset.result = "failed";
    console.error(error);
  }
}

if (params.has("test")) await runChecks();
else {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: false });
  results.textContent = "点击会话把 Cindy 拿到前面。";
}
