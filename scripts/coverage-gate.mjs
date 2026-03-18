#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";

const [, , summaryPath, ...thresholdArgs] = process.argv;

if (!summaryPath) {
  console.error(
    "usage: node scripts/coverage-gate.mjs <coverage-summary.json> [crate=min_percent ...]",
  );
  process.exit(2);
}

const summary = JSON.parse(fs.readFileSync(summaryPath, "utf8"));
const files = summary?.data?.[0]?.files ?? [];
const crates = ["ferrocat", "ferrocat-po", "ferrocat-icu"];
const thresholds = new Map();

for (const arg of thresholdArgs) {
  const [crate, value] = arg.split("=");
  if (!crate || value === undefined) {
    console.error(`invalid threshold argument: ${arg}`);
    process.exit(2);
  }
  thresholds.set(crate, Number.parseFloat(value));
}

let hasFailure = false;
for (const crate of crates) {
  let covered = 0;
  let count = 0;
  const marker = `${path.sep}crates${path.sep}${crate}${path.sep}`;

  for (const file of files) {
    if (!file.filename.includes(marker)) {
      continue;
    }
    covered += file.summary.lines.covered;
    count += file.summary.lines.count;
  }

  if (count === 0) {
    console.log(`${crate}: N/A (no measurable executable lines in llvm-cov report)`);
    continue;
  }

  const percent = (covered / count) * 100;
  const formatted = percent.toFixed(2);
  const threshold = thresholds.get(crate);

  if (threshold === undefined) {
    console.log(`${crate}: ${formatted}% (${covered}/${count})`);
    continue;
  }

  const status = percent + 1e-9 >= threshold ? "PASS" : "FAIL";
  console.log(
    `${crate}: ${formatted}% (${covered}/${count}) against ${threshold.toFixed(2)}% => ${status}`,
  );
  if (status === "FAIL") {
    hasFailure = true;
  }
}

process.exit(hasFailure ? 1 : 0);
