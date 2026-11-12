import test from "node:test";
import assert from "node:assert/strict";
import { createColorQueue } from "../src/color-live.js";

function clock() {
  const timers = new Map();
  let next = 0;
  return {
    timers,
    schedule(fn) { const id = ++next; timers.set(id, fn); return id; },
    unschedule(id) { timers.delete(id); },
    fire() {
      const [id, fn] = timers.entries().next().value;
      timers.delete(id);
      fn();
    },
  };
}
const settle = () => new Promise(resolve => setImmediate(resolve));

test("idle queue has no timers, writes or recurring work", () => {
  const c = clock();
  let calls = 0;
  createColorQueue(() => { calls++; }, c);
  assert.equal(c.timers.size, 0);
  assert.equal(calls, 0);
});

test("rapid edits combine both controls without postponing the live update", async () => {
  const c = clock(), calls = [];
  const q = createColorQueue(value => { calls.push(value); }, c);
  const first = q.enqueue([1.1, 1]);
  const timer = [...c.timers.keys()][0];
  const last = q.enqueue([1.3, 1.25]);
  assert.equal((await first).status, "superseded");
  assert.deepEqual([...c.timers.keys()], [timer]);
  c.fire();
  assert.equal((await last).status, "applied");
  await settle();
  assert.deepEqual(calls, [[1.3, 1.25]]);
  assert.equal(c.timers.size, 0);
});

test("slow driver receives the latest edit with no overlapping calls", async () => {
  const c = clock(), calls = [], finish = [];
  let active = 0, peak = 0;
  const q = createColorQueue(value => {
    calls.push(value);
    peak = Math.max(peak, ++active);
    return new Promise(resolve => finish.push(() => { active--; resolve(); }));
  }, c);
  const first = q.enqueue([1.1, 1]);
  c.fire(); await settle();
  const middle = q.enqueue([1.4, 1.1]);
  const last = q.enqueue([1.8, 1.3]);
  c.fire();
  assert.equal((await middle).status, "superseded");
  assert.equal(calls.length, 1);
  finish.shift()(); await settle();
  assert.deepEqual(calls, [[1.1, 1], [1.8, 1.3]]);
  finish.shift()();
  assert.equal((await first).current, false);
  assert.equal((await last).current, true);
  assert.equal(peak, 1);
});

test("presets replace queued slider work and apply immediately", async () => {
  const c = clock(), calls = [];
  const q = createColorQueue(value => { calls.push(value); }, c);
  const old = q.enqueue([1.2, 1.1]);
  const preset = q.enqueue([2, 1], true);
  assert.equal(c.timers.size, 0);
  assert.equal((await old).status, "superseded");
  assert.equal((await preset).status, "applied");
  assert.deepEqual(calls, [[2, 1]]);
});

test("cancelling pending edits leaves the display untouched", async () => {
  const c = clock(); let writes = 0;
  const q = createColorQueue(() => { writes++; }, c);
  const pending = q.enqueue([1.5, 1.2]);
  await q.cancel();
  assert.equal((await pending).status, "cancelled");
  assert.equal(c.timers.size, 0);
  assert.equal(writes, 0);
});

test("recovery waits for an active call and discards stale pending edits", async () => {
  const c = clock(), calls = []; let finish;
  const q = createColorQueue(value => {
    calls.push(value);
    return new Promise(resolve => { finish = resolve; });
  }, c);
  const active = q.enqueue([1.2, 1.1], true);
  await settle();
  const pending = q.enqueue([1.9, 1.5]);
  let restored = false;
  const recovery = q.cancel().then(() => { restored = true; });
  await settle();
  assert.equal(restored, false);
  finish(); await recovery;
  assert.equal((await active).current, false);
  assert.equal((await pending).status, "cancelled");
  assert.equal(restored, true);
  assert.equal(calls.length, 1);
  assert.equal(c.timers.size, 0);
});

test("driver errors are reported without automatic retry or idle timers", async () => {
  const c = clock(); let attempts = 0;
  const q = createColorQueue(() => { attempts++; throw new Error("driver refused"); }, c);
  const result = await q.enqueue([1.2, 1.1], true);
  await settle();
  assert.equal(result.status, "failed");
  assert.match(result.error, /driver refused/);
  assert.equal(attempts, 1);
  assert.equal(c.timers.size, 0);
});
