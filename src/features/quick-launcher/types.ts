/** A launcher item as the native index returns it. */
export type LauncherEntryKind = "app" | "file";

export type LauncherEntry = {
  id: string;
  name: string;
  /** Where it is: a target path, a URL, or a folder. */
  subtitle: string;
  /** What opening it runs. */
  target: string;
  kind: LauncherEntryKind;
};

/** How an entry matched the query, best first. */
export type LauncherMatch = "exact" | "prefix" | "word" | "contains" | "path" | "web";

/** One row in the result list: an index entry or the web-search fallback. */
export type LauncherResult = {
  id: string;
  name: string;
  subtitle: string;
  target: string;
  kind: LauncherEntryKind | "web";
  match: LauncherMatch;
};

export type SearchEngine = "bing" | "google" | "baidu" | "duckduckgo";
