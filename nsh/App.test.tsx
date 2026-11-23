// The Rust root tested from TypeScript: `nsh test` runs this file in a
// headless runner it builds with the crate, and `open(".")` starts the root
// (`nsh.json` "root").
import { expect, open, test } from "nsh/test";

test("opens as a collapsed capsule and expands", async () => {
  const app = await open(".");
  const texts = app.texts().join("\n");
  const title = texts.includes("焦点岛") ? "焦点岛" : "FocuSD Island";
  expect(texts).toContain(title);
  await app.press(title);
  const after = app.texts().join("\n");
  expect(after.includes("复制") || after.includes("COPY")).toBe(true);
});
