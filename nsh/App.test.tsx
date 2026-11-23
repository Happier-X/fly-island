// The Rust root tested from TypeScript: `nsh test` runs this file in a
// headless runner it builds with the crate, and `open(".")` starts the root
// (`nsh.json` "root").
import { expect, open, test } from "nsh/test";

test("opens as a collapsed capsule and expands", async () => {
  const app = await open(".");
  expect(app.texts().join("\n")).toContain("FocuSD Island");
  await app.press("FocuSD Island");
  expect(app.texts().join("\n")).toContain("Clipboard history");
  expect(app.texts().join("\n")).toContain("Copy some text to see it here");
});
