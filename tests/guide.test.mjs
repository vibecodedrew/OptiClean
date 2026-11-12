import test from "node:test";
import assert from "node:assert/strict";
import { Profile, empty, ready, missing, sections } from "../src/out/Guide.js";

const desktop = overrides => Object.assign(new Profile(
  "AMD Ryzen 7 7700X", "NVIDIA GeForce RTX 4070", "Example B650 Board", "1.20",
  "Reference PSU Model", "750", "Reference Tower Cooler", "70", "65", true, true, true, true
), overrides);
const plan = p => Array.from(sections(p)).at(-1);
const text = p => Array.from(plan(p).items, i => i.set).join(" ");

test("missing hardware never produces a clock recommendation", () => {
  assert.equal(ready(empty), false);
  assert.ok(Array.from(missing(empty)).includes("PSU make/model"));
  assert.doesNotMatch(text(empty), /\+15 MHz|negative magnitude/);
});
test("validated desktop gets bounded manual steps with stock power", () => {
  const p = desktop();
  assert.equal(ready(p), true);
  assert.match(text(p), /negative magnitude of 5 only/);
  assert.match(text(p), /\+15 MHz core, \+0 memory, stock voltage and stock power limit/);
});
test("unsupported chipset and X3D never receive generic Curve Optimizer", () => {
  for (const p of [desktop({board: "Example A520 Board"}), desktop({cpu: "AMD Ryzen 7 7800X3D"})]) {
    assert.doesNotMatch(text(p), /negative magnitude/);
    assert.match(text(p), /factory automatic boost/);
  }
});
test("thermal headroom and all confirmations are required", () => {
  for (const values of [{cpuTemp:"81"},{gpuTemp:"76"},{headroom:false},{support:false},{desktop:false},{stable:false}]) {
    const p=desktop(values);
    assert.equal(ready(p),false);
    assert.doesNotMatch(text(p), /\+15 MHz|negative magnitude/);
  }
});
test("invalid measurements and placeholder identities remain unknown", () => {
  for (const values of [{cpuTemp:"NaN"},{gpuTemp:"Infinity"},{watts:"0"},{psu:"Unknown"},{board:"To be filled by O.E.M."}]) {
    assert.equal(ready(desktop(values)),false);
  }
});
test("mobile model names override the desktop checkbox", () => {
  for (const cpu of ["Intel Core i7-13700H", "AMD Ryzen 7 7840HS"]) {
    assert.doesNotMatch(text(desktop({cpu})), /\+15 MHz|negative magnitude/);
  }
});
test("unknown GPU and locked Intel CPU stay at factory settings", () => {
  const p=desktop({cpu:"Intel Core i5-13400",gpu:"Intel UHD Graphics 730",board:"Reference B760 Board"});
  assert.match(text(p), /GPU tuning support is unverified/);
  assert.doesNotMatch(text(p), /\+15 MHz|negative magnitude/);
});
