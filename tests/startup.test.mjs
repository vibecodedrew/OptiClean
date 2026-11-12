import test from "node:test";
import assert from "node:assert/strict";
import { revealWhenMounted } from "../src/startup.js";

function fixture(mounted = false) {
  const root = { mounted, hasChildNodes() { return this.mounted; } };
  let observer;
  class Observer {
    constructor(callback) { this.callback = callback; this.disconnected = false; observer = this; }
    observe() {}
    disconnect() { this.disconnected = true; }
  }
  let shows = 0;
  revealWhenMounted(root, () => { shows++; }, Observer);
  return { root, get observer() { return observer; }, get shows() { return shows; } };
}

test("native window stays hidden until content mounts, without animation frames", async () => {
  const f = fixture();
  await Promise.resolve();
  assert.equal(f.shows, 0);
  f.root.mounted = true;
  f.observer.callback();
  await Promise.resolve();
  assert.equal(f.shows, 1);
  assert.equal(f.observer.disconnected, true);
});
test("subsequent hardware or status updates cannot show the window again", async () => {
  const f = fixture(true);
  f.observer.callback();
  f.observer.callback();
  await Promise.resolve();
  assert.equal(f.shows, 1);
});
