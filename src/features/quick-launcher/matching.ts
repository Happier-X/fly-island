import type {
  LauncherEntry,
  LauncherMatch,
  LauncherResult,
  SearchEngine,
} from "./types.ts";

/** Rows the panel shows at once; the rest are reachable by typing more. */
export const DEFAULT_RESULT_LIMIT = 30;

const MAX_RESULT_LIMIT = 100;

/** Where one match stands: exact name, name prefix, a word's start, inside the name, or the path. */
const MATCH_ORDER: Record<LauncherMatch, number> = {
  exact: 0,
  prefix: 1,
  word: 2,
  contains: 3,
  path: 4,
  web: 5,
};

/** Characters that start a new word in an application or file name. */
const WORD_SEPARATOR = /[\s\-_./\\(){}，、,]+/;

export function normalizeQuery(query: string) {
  return query.trim().toLocaleLowerCase();
}

function nameMatch(name: string, query: string): LauncherMatch | null {
  const lower = name.toLocaleLowerCase();
  if (lower === query) return "exact";
  if (lower.startsWith(query)) return "prefix";
  let at = lower.indexOf(query);
  if (at < 0) return null;
  while (at >= 0) {
    if (WORD_SEPARATOR.test(lower[at - 1])) return "word";
    at = lower.indexOf(query, at + 1);
  }
  return "contains";
}

function toResult(entry: LauncherEntry, match: LauncherMatch): LauncherResult {
  return { ...entry, match };
}

function compareNames(a: LauncherResult, b: LauncherResult) {
  const left = a.name.toLocaleLowerCase();
  const right = b.name.toLocaleLowerCase();
  return left < right ? -1 : left > right ? 1 : 0;
}

/**
 * Ranks entries against a query. A name equal to it comes first, then names that start
 * with it, then names where a word starts with it, then names containing it, and files
 * whose folder path contains it last. Within a tier apps precede files, then shorter names come first and ties
 * are broken by name and id, so the order is stable and testable.
 *
 * An empty query matches nothing. The web result is offered only when nothing else
 * matches, so it can never hide an application or file the user meant.
 */
export function searchLauncher(
  entries: readonly LauncherEntry[],
  query: string,
  options: { limit?: number; searchWeb?: boolean; engine?: SearchEngine } = {},
): LauncherResult[] {
  const normalized = normalizeQuery(query);
  if (!normalized) return [];
  const limit = Math.min(Math.max(Math.trunc(options.limit ?? DEFAULT_RESULT_LIMIT), 1), MAX_RESULT_LIMIT);

  const matched: LauncherResult[] = [];
  for (const entry of entries) {
    const match =
      nameMatch(entry.name, normalized) ??
      (entry.kind === "file" && entry.subtitle.toLocaleLowerCase().includes(normalized)
        ? "path"
        : null);
    if (match) matched.push(toResult(entry, match));
  }
  matched.sort(
    (a, b) =>
      MATCH_ORDER[a.match] - MATCH_ORDER[b.match] ||
      Number(a.kind !== "app") - Number(b.kind !== "app") ||
      a.name.length - b.name.length ||
      compareNames(a, b) ||
      (a.id < b.id ? -1 : a.id > b.id ? 1 : 0),
  );

  const seen = new Set<string>();
  const results: LauncherResult[] = [];
  for (const result of matched) {
    const key = `${result.kind}:${result.target.toLocaleLowerCase()}`;
    if (seen.has(key)) continue;
    seen.add(key);
    results.push(result);
    if (results.length >= limit) break;
  }

  if (results.length === 0 && options.searchWeb) {
    const engine = options.engine ?? "bing";
    results.push({
      id: "web",
      name: query.trim(),
      subtitle: searchEngineHost(engine),
      target: searchEngineUrl(engine, query.trim()),
      kind: "web",
      match: "web",
    });
  }
  return results;
}

const ENGINE_URLS: Record<SearchEngine, (query: string) => string> = {
  bing: (query) => `https://www.bing.com/search?q=${encodeURIComponent(query)}`,
  google: (query) => `https://www.google.com/search?q=${encodeURIComponent(query)}`,
  baidu: (query) => `https://www.baidu.com/s?wd=${encodeURIComponent(query)}`,
  duckduckgo: (query) => `https://duckduckgo.com/?q=${encodeURIComponent(query)}`,
};

const ENGINE_HOSTS: Record<SearchEngine, string> = {
  bing: "www.bing.com",
  google: "www.google.com",
  baidu: "www.baidu.com",
  duckduckgo: "duckduckgo.com",
};

export function searchEngineUrl(engine: SearchEngine, query: string) {
  return ENGINE_URLS[engine](query);
}

export function searchEngineHost(engine: SearchEngine) {
  return ENGINE_HOSTS[engine];
}
