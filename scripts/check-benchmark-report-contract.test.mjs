import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { validateReport } from "./check-benchmark-report-contract.mjs";

const report = JSON.parse(
  await readFile(
    new URL("../benchmark/results/gettext-official-v1-c3caa27.json", import.meta.url),
    "utf8",
  ),
);

test("historical report and additive fields satisfy the contract", () => {
  const extended = structuredClone(report);
  extended.future_field = true;
  extended.scenarios[0].statistics.future_metric = 1;
  assert.doesNotThrow(() => validateReport(extended));
});

test("missing provenance and invalid metrics are rejected", () => {
  const missing = structuredClone(report);
  delete missing.environment.git_sha;
  assert.throws(() => validateReport(missing), /missing git_sha/);
  const invalid = structuredClone(report);
  invalid.scenarios[0].statistics.noisy = "false";
  assert.throws(() => validateReport(invalid), /expected boolean/);
  const overflow = structuredClone(report);
  overflow.scenarios[0].statistics.median_mib_per_sec = JSON.parse("1e999");
  assert.throws(() => validateReport(overflow), /expected number/);
});
