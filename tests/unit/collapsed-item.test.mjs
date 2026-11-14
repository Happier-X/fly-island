import assert from "node:assert/strict";
import test from "node:test";
import { pickCollapsedItem } from "../../src/windows/island/collapsedItem.ts";

const TODO_FIRST = ["todo", "lyrics"];
const DEFAULT_ORDER = ["lyrics", "todo"];

test("a started todo ranked above the lyric line is what the collapsed island shows", () => {
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: true, lyrics: true }, true), "todo");
  // Ranked below, it still waits its turn.
  assert.equal(pickCollapsedItem(DEFAULT_ORDER, { todo: true, lyrics: true }, true), "lyrics");
});

test("with no todo started, the remaining count gives way to a lyric line", () => {
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: true, lyrics: true }, false), "lyrics");
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: true, lyrics: false }, false), "todo");
});

test("an unstarted todo still shows when nothing else has anything", () => {
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: true, lyrics: false }, false), "todo");
  assert.equal(pickCollapsedItem(DEFAULT_ORDER, { todo: true, lyrics: false }, false), "todo");
});

test("nothing to show leaves the text place empty", () => {
  assert.equal(pickCollapsedItem(TODO_FIRST, { todo: false, lyrics: false }, false), null);
  assert.equal(pickCollapsedItem(DEFAULT_ORDER, { todo: false, lyrics: false }), null);
});
