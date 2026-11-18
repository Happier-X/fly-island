// pnpm dev, then open /tests/regression/launcher.html. Without ?test=1 the panel is shown
// with the fixture below for visual QA. With ?test=1 it checks filtering and ranking, the
// web fallback, arrow-key selection with wrapping and scrolling, Enter, Escape, clicks,
// input-method composition and the empty state.
import "../../src/styles/tokens.css";
import "../../src/styles/base.css";
import { act, StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider, LOCALE_STORAGE_KEY } from "../../src/i18n";
import { LauncherView } from "../../src/features/quick-launcher/LauncherView";
import {
  DEFAULT_LAUNCHER_SETTINGS,
  type LauncherSettings,
} from "../../src/features/quick-launcher/storage";
import type { LauncherEntry, LauncherResult } from "../../src/features/quick-launcher/types";

const query = new URLSearchParams(location.search);
const automated = query.has("test");
Object.assign(globalThis, { isTauri: true, IS_REACT_ACT_ENVIRONMENT: automated });
localStorage.setItem(LOCALE_STORAGE_KEY, query.get("locale") === "en" ? "en" : "zh-CN");

const ICON = `data:image/svg+xml,${encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="24" height="24" rx="5" fill="#3a7bd5"/></svg>')}`;

const entry = (id: string, name: string, extra: Partial<LauncherEntry> = {}): LauncherEntry => ({
  id,
  name,
  subtitle: `C:\\Apps\\${name}.exe`,
  target: `C:\\Apps\\${name}.exe`,
  kind: "app",
  ...extra,
});

const PHOTO_APPS = [
  entry("app:photoshop", "Photoshop"),
  entry("app:photo", "Photo"),
  entry("app:adobe", "Adobe Photoshop"),
  entry("app:telephoto", "Telephoto"),
  entry("app:editor", "Photo Editor"),
];
const PROJECTS = Array.from({ length: 24 }, (_, index) =>
  entry(`file:project-${index}`, `Project ${String(index + 1).padStart(2, "0")}.psd`, {
    kind: "file",
    subtitle: "D:\\Design\\Projects",
    target: `D:\\Design\\Projects\\Project ${index + 1}.psd`,
  }),
);
const ENTRIES = [...PHOTO_APPS, ...PROJECTS];

const settings: LauncherSettings = {
  ...DEFAULT_LAUNCHER_SETTINGS,
  searchWeb: true,
  searchEngine: "bing",
};

const opened: LauncherResult[] = [];
let closed = 0;
const icons = { "app:photoshop": ICON };
const stableResults = () => undefined;

let resetPanel = () => {};
function Harness() {
  const [openToken, setOpenToken] = useState(0);
  const [visible, setVisible] = useState(true);
  // Re-exposed to the checks so each case can start from a fresh open.
  resetPanel = () => {
    setVisible(false);
    setOpenToken((value) => value + 1);
  };
  return (
    <LauncherView
      entries={ENTRIES}
      settings={settings}
      icons={icons}
      loading={visible}
      openToken={openToken}
      openError=""
      onOpen={(result) => {
        opened.push(result);
        setVisible(false);
      }}
      onClose={() => {
        closed += 1;
        setVisible(false);
      }}
      onResultsChange={stableResults}
    />
  );
}

const fixture = document.getElementById("fixture")!;
const resultsBox = document.getElementById("results")!;
const root = createRoot(fixture);

if (!automated) {
  root.render(
    <StrictMode>
      <I18nProvider>
        <Harness />
      </I18nProvider>
    </StrictMode>,
  );
  resultsBox.remove();
} else {
  const messages: string[] = [];
  const check = (condition: unknown, label: string) => {
    if (!condition) throw new Error(label);
    messages.push(`PASS ${label}`);
  };
  const input = () => document.querySelector<HTMLInputElement>(".launcher-search__input")!;
  const rows = () => [...document.querySelectorAll<HTMLElement>(".launcher-result")];
  const rowNames = () =>
    rows().map((row) => row.querySelector(".launcher-result__name")?.textContent ?? "");
  const selectedRow = () => document.querySelector<HTMLElement>('.launcher-result[data-selected="true"]');
  const type = async (value: string) => {
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    await act(async () => {
      setter.call(input(), value);
      input().dispatchEvent(new Event("input", { bubbles: true }));
    });
  };
  const press = async (key: string, extra: KeyboardEventInit = {}) => {
    await act(async () => {
      input().dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...extra }));
    });
  };
  const reopen = async () => {
    resetPanel();
    await act(async () => {});
  };

  try {
    await act(async () => {
      root.render(
        <StrictMode>
          <I18nProvider>
            <Harness />
          </I18nProvider>
        </StrictMode>,
      );
    });
    check(Boolean(input()) && input() === document.activeElement, "打开后搜索框自动获得焦点");
    const panelStyle = getComputedStyle(document.querySelector<HTMLElement>(".launcher-window")!);
    check(panelStyle.boxShadow === "none" && panelStyle.borderTopWidth === "1px",
      "面板没有外扩阴影，只留细边框做区分");
    check(document.querySelector(".launcher-empty")?.textContent?.includes("输入名称"), "空查询显示输入提示，不列出全部");
    check(rows().length === 0 && document.querySelector(".launcher-footer__status")?.textContent?.includes("正在加载"), "空列表时底部说明正在加载");

    await type("weather report");
    check(rows().length === 0, "本地索引加载期间不提前提供网页兜底");
    await act(async () => { resetPanel(); });

    // Ranking: exact, name prefix (shortest first), word prefix, contains.
    await type("photo");
    check(rowNames().join(" | ") === "Photo | Photoshop | Photo Editor | Adobe Photoshop | Telephoto",
      "排序为完全匹配 > 名称前缀 > 单词前缀 > 包含，同层短的在前");
    check(selectedRow()?.dataset.launcherId === "app:photo", "输入后默认高亮第一行");
    check(document.querySelector<HTMLImageElement>('[data-launcher-id="app:photoshop"] img')?.src === ICON,
      "有系统图标的结果显示图标");
    check(Boolean(document.querySelector('[data-launcher-id="app:photo"] .lucide-app-window')), "没有图标时用通用应用图标");

    // Arrow keys wrap and move the highlight.
    await press("ArrowDown");
    check(selectedRow()?.dataset.launcherId === "app:photoshop", "↓ 移到下一行");
    await press("ArrowUp");
    await press("ArrowUp");
    check(selectedRow()?.dataset.launcherId === "app:telephoto", "在第一行按 ↑ 回到最后一行");
    await press("ArrowDown");
    check(selectedRow()?.dataset.launcherId === "app:photo", "在最后一行按 ↓ 回到第一行");

    await press("Enter");
    check(opened[opened.length - 1]?.name === "Photo" && closed === 0, "回车打开高亮项，不关闭面板");

    await press("ArrowDown");
    await press("Enter");
    check(opened[opened.length - 1]?.name === "Photoshop", "回车打开的是当前高亮项");

    await press("Escape");
    check(closed === 1, "Esc 请求关闭面板");

    // A click opens its row.
    await reopen();
    await type("photo");
    await act(async () => {
      rows().find((row) => row.dataset.launcherId === "app:editor")!.click();
    });
    check(opened[opened.length - 1]?.name === "Photo Editor", "点击结果打开它");

    // Enter during input-method composition belongs to the composition.
    await reopen();
    await type("photo");
    const beforeComposition = opened.length;
    await press("Enter", { isComposing: true });
    check(opened.length === beforeComposition, "输入法组合中按回车不会打开结果");

    // Web fallback only when nothing matches.
    await reopen();
    await type("weather report");
    check(rows().length === 1 && rows()[0].dataset.launcherId === "web", "没有匹配时给出网页搜索兜底");
    check(rowNames()[0] === "在网页中搜索“weather report”" && rows()[0].querySelector(".launcher-result__subtitle")?.textContent === "www.bing.com",
      "兜底行说明搜索词和搜索引擎");
    await press("Enter");
    check(opened[opened.length - 1]?.kind === "web" && opened[opened.length - 1]?.target === "https://www.bing.com/search?q=weather%20report",
      "回车打开默认搜索引擎的网页");

    await reopen();
    await type("photo");
    check(rows().every((row) => row.dataset.launcherId !== "web"), "有匹配时不显示网页兜底");

    // A long list scrolls the highlighted row into view.
    await reopen();
    await type("project");
    check(rows().length === 24, "匹配结果按上限列出");
    for (let step = 0; step < 14; step += 1) await press("ArrowDown");
    const list = document.querySelector<HTMLElement>(".launcher-results")!;
    const chosen = selectedRow()!;
    const listBox = list.getBoundingClientRect();
    const chosenBox = chosen.getBoundingClientRect();
    check(chosen.dataset.launcherId === "file:project-14", "连续按 ↓ 逐行移动");
    check(chosenBox.top >= listBox.top - 1 && chosenBox.bottom <= listBox.bottom + 1 && list.scrollTop > 0,
      "高亮行超出可见范围时列表自动滚动");

    // Clearing the field returns to the hint.
    await type("");
    check(rows().length === 0 && document.querySelector(".launcher-empty")?.textContent?.includes("输入名称"), "清空输入后回到提示状态");

    // English copy comes from the dictionary too.
    resultsBox.textContent = `${messages.join("\n")}\n\n全部 ${messages.length} 项检查通过。`;
    resultsBox.dataset.result = "passed";
  } catch (error) {
    resultsBox.textContent = `${messages.join("\n")}\nFAIL ${String(error)}`;
    resultsBox.dataset.result = "failed";
  }
}
