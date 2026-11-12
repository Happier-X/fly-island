import assert from "node:assert/strict";
import test from "node:test";
import { sessionLight } from "../../src/features/agent-status/sessionLight.ts";

const light = (phase) => sessionLight({ phase });

test("each phase has its own light", () => {
  assert.equal(light("waiting"), "needs");
  assert.equal(light("running"), "working");
  assert.equal(light("stale"), "stale");
  assert.equal(light("failed"), "failed");
  assert.equal(light("completed"), "done");
});
