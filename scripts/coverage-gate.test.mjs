import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const gate = fileURLToPath(new URL("./coverage-gate.mjs", import.meta.url));
const floors = ["ferrocat-cli=85", "ferrocat-po=95", "ferrocat-icu=95"];
const file = (crate, covered, count) => ({
  filename: path.resolve("crates", crate, "src", "lib.rs"),
  summary: { lines: { covered, count } },
});
const measured = [
  file("ferrocat-cli", 85, 100),
  file("ferrocat-po", 96, 100),
  file("ferrocat-icu", 95, 100),
];

function run(files) {
  const dir = mkdtempSync(path.join(tmpdir(), "ferrocat-coverage-test-"));
  try {
    const summary = path.join(dir, "coverage-summary.json");
    writeFileSync(summary, JSON.stringify({ data: [{ files }] }));
    return spawnSync(process.execPath, [gate, summary, ...floors], { encoding: "utf8" });
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

test("passes measured crates at their floor with optional umbrella absent", () => {
  const result = run(measured);
  assert.equal(result.status, 0);
  assert.match(result.stdout, /ferrocat: N\/A/);
  assert.match(result.stdout, /ferrocat-cli: 85\.00% \(85\/100\) against 85\.00% => PASS/);
  assert.match(result.stdout, /ferrocat-icu: 95\.00% \(95\/100\) against 95\.00% => PASS/);
});

for (const [name, po] of [
  ["is absent", []],
  ["has zero measurable lines", [file("ferrocat-po", 0, 0)]],
]) {
  test(`fails when a crate with a coverage floor ${name}`, () => {
    const result = run([file("ferrocat-cli", 85, 100), ...po, file("ferrocat-icu", 95, 100)]);
    assert.equal(result.status, 1);
    assert.match(result.stdout, /ferrocat-po: N\/A.*95\.00%.*FAIL/);
    assert.match(result.stdout, /ferrocat-cli:.*PASS/);
    assert.match(result.stdout, /ferrocat-icu:.*PASS/);
  });
}

test("zero covered lines with measurable lines is a numerical failure", () => {
  const result = run([
    file("ferrocat-cli", 85, 100),
    file("ferrocat-po", 0, 100),
    file("ferrocat-icu", 95, 100),
  ]);
  assert.equal(result.status, 1);
  assert.match(result.stdout, /ferrocat-po: 0\.00% \(0\/100\) against 95\.00% => FAIL/);
});

test("fails all enforced crates when the report has no files", () => {
  const result = run([]);
  assert.equal(result.status, 1);
  assert.match(result.stdout, /ferrocat: N\/A/);
  for (const crate of ["ferrocat-cli", "ferrocat-po", "ferrocat-icu"]) {
    assert.match(result.stdout, new RegExp(`${crate}: N/A.*FAIL`));
  }
});
