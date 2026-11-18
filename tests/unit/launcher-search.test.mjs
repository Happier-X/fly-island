import assert from "node:assert/strict";
import { test } from "node:test";
import {
  normalizeQuery,
  searchEngineUrl,
  searchLauncher,
} from "../../src/features/quick-launcher/matching.ts";

const entry = (name, extra = {}) => ({
  id: `app:${name.toLowerCase()}`,
  name,
  subtitle: `C:\\Program Files\\${name}.exe`,
  target: `C:\\Program Files\\${name}.exe`,
  kind: "app",
  ...extra,
});

const names = (results) => results.map((result) => result.name);

test("an empty query matches nothing", () => {
  const entries = [entry("Photoshop")];
  assert.deepEqual(searchLauncher(entries, ""), []);
  assert.deepEqual(searchLauncher(entries, "   "), []);
  assert.equal(normalizeQuery("  Photo  "), "photo");
});

test("results rank exact, then name prefix, then word prefix, then contains", () => {
  const entries = [
    entry("Telephoto"),        // contains
    entry("Adobe Photoshop"),  // word prefix
    entry("Photoshop"),        // name prefix
    entry("Photo"),            // exact
  ];
  const results = searchLauncher(entries, "photo");
  assert.deepEqual(names(results), ["Photo", "Photoshop", "Adobe Photoshop", "Telephoto"]);
  assert.deepEqual(results.map((result) => result.match), ["exact", "prefix", "word", "contains"]);
});

test("matching ignores case and punctuation-separated words count as word starts", () => {
  const entries = [entry("visual-studio-code"), entry("My_code_tool")];
  const results = searchLauncher(entries, "CODE");
  // Both are word starts; the shorter name comes first within the tier.
  assert.deepEqual(names(results), ["My_code_tool", "visual-studio-code"]);
  assert.deepEqual(results.map((result) => result.match), ["word", "word"]);
});

test("files can match on their folder path, after every name match", () => {
  const entries = [
    entry("notes.pdf", { kind: "file", subtitle: "D:\\Photos", id: "file:notes" }),
    entry("Photo.pdf", { kind: "file", subtitle: "D:\\Docs", id: "file:photo" }),
  ];
  const results = searchLauncher(entries, "photo");
  assert.deepEqual(names(results), ["Photo.pdf", "notes.pdf"]);
  assert.equal(results[1].match, "path");
  // The subtitle of an application is its executable path, not a searchable folder.
  const app = [entry("Notepad", { subtitle: "C:\\Photo\\Notepad.exe" })];
  assert.deepEqual(searchLauncher(app, "photo"), []);
});

test("later results are cut at the limit and duplicates by target appear once", () => {
  const many = Array.from({ length: 6 }, (_, index) => entry(`Photo ${index}`));
  assert.equal(searchLauncher(many, "photo", { limit: 2 }).length, 2);
  const duplicates = [
    entry("Widget", { target: "C:\\Widget.exe" }),
    entry("Widget", { id: "app:other", target: "c:\\widget.exe" }),
  ];
  assert.equal(searchLauncher(duplicates, "widget").length, 1);
});

test("the web fallback appears only when nothing else matches", () => {
  const entries = [entry("Photoshop")];
  const fallback = searchLauncher(entries, "weather report", { searchWeb: true, engine: "bing" });
  assert.equal(fallback.length, 1);
  assert.equal(fallback[0].kind, "web");
  assert.equal(fallback[0].match, "web");
  assert.equal(fallback[0].subtitle, "www.bing.com");
  assert.equal(fallback[0].target, "https://www.bing.com/search?q=weather%20report");

  assert.deepEqual(searchLauncher(entries, "photo", { searchWeb: true }), [
    { ...entry("Photoshop"), match: "prefix" },
  ]);
  assert.deepEqual(searchLauncher([], "weather", { searchWeb: false }), []);
});

test("each engine has its own search URL", () => {
  assert.equal(searchEngineUrl("google", "a b"), "https://www.google.com/search?q=a%20b");
  assert.equal(searchEngineUrl("baidu", "天气"), "https://www.baidu.com/s?wd=%E5%A4%A9%E6%B0%94");
  assert.equal(searchEngineUrl("duckduckgo", "rust"), "https://duckduckgo.com/?q=rust");
});

test("Chinese names match by the same tiers without pinyin", () => {
  const entries = [entry("网易云音乐"), entry("云音乐"), entry("音乐播放器")];
  assert.deepEqual(names(searchLauncher(entries, "云音乐")), ["云音乐", "网易云音乐"]);
  assert.deepEqual(names(searchLauncher(entries, "音乐")), ["音乐播放器", "云音乐", "网易云音乐"]);
});
