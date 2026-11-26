/* eslint-disable security/detect-non-literal-fs-filename -- Only fixed, repository-owned paths are read or written. */
/**
 * Writes app/data/home-samples.json, everything the homepage shows besides the
 * CLI audit (render-audit-sample.mjs):
 *
 * - the catalog, ICU, review and runtime specimens, printed by docs/samples
 *   using Ferrocat from this workspace;
 * - the conformance counts, from the repository's harness
 *   (`cargo run -p ferrocat-bench -- conformance-report`);
 * - the speed comparison, read from the committed benchmark reports.
 *
 * `pnpm sample:write` writes the file, `pnpm sample:check` fails when it no
 * longer matches. Nobody edits the file by hand.
 */
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { format } from "oxfmt";

const root = fileURLToPath(new URL("../../", import.meta.url));
const check = process.argv.includes("--check");
const destination = new URL("../app/data/home-samples.json", import.meta.url);
const readJson = async (path) => JSON.parse(await readFile(new URL(path, import.meta.url), "utf8"));

// Generate the specimens with the same workspace code as the conformance report.
const specimens = JSON.parse(
  execFileSync("cargo", ["run", "--locked", "--quiet", "-p", "ferrocat-homepage-samples"], {
    cwd: root,
    encoding: "utf8",
  }),
);

// The conformance counts, as the repository's harness reports them.
const report = execFileSync(
  "cargo",
  ["run", "--locked", "--quiet", "-p", "ferrocat-bench", "--", "conformance-report"],
  { cwd: root, encoding: "utf8" },
);
const field = (text, name) => {
  const line = text.split("\n").find((entry) => entry.startsWith(`${name}: `));
  assert(line, `The conformance report has no ${name}.`);
  return Number(line.slice(name.length + 2));
};
const suites = report
  .split(/\n{2,}/)
  .filter((block) => block.startsWith("suite: "))
  .map((block) => ({
    id: /^suite: (.+)$/m.exec(block)?.[1],
    cases: field(block, "cases"),
    assertions: field(block, "assertions"),
  }));
const conformance = {
  cases: field(report, "total-cases"),
  assertions: field(report, "total-assertions"),
  expectedPass: field(report, "expected-pass"),
  expectedReject: field(report, "expected-reject"),
  knownGaps: field(report, "known-gap"),
  failed: field(report, "failed-cases"),
  suites,
};
assert.equal(conformance.failed, 0, "The conformance harness reports failing cases.");

// The speed comparison: median throughput from the committed reports, turned
// into time relative to Ferrocat within the same report.
const reports = {
  official: await readJson("../../benchmark/results/gettext-official-v1-1aee601.json"),
  workflows: await readJson("../../benchmark/results/gettext-workflows-ecosystem-v1-1aee601.json"),
};
const tasks = [
  {
    id: "parse",
    group: "po-parse/gettext-ui-de-10000",
    ferrocat: { report: "official", implementation: "ferrocat" },
    comparators: [
      { report: "official", implementation: "gettext-parser", label: "gettext-parser" },
      { report: "official", implementation: "php-gettext", label: "gettext/gettext" },
      { report: "official", implementation: "polib", label: "polib" },
    ],
  },
  {
    id: "merge",
    group: "po-merge/gettext-ui-de-10000",
    ferrocat: { report: "workflows", implementation: "ferrocat" },
    comparators: [
      { report: "workflows", implementation: "msgmerge", label: "GNU msgmerge" },
      { report: "workflows", implementation: "polib", label: "polib" },
      { report: "official", implementation: "php-gettext", label: "gettext/gettext" },
    ],
  },
];
function scenario(source, group, implementation) {
  // A report may list a scenario twice; the first entry is the published one.
  const entry = reports[source].scenarios.find(
    (candidate) => candidate.id === `${group}/${implementation}`,
  );
  assert(entry, `${source} has no ${group}/${implementation}.`);
  const stats = entry.statistics;
  // Command-line tools are measured with their process start subtracted.
  const throughput = stats.median_adjusted_mib_per_sec ?? stats.median_mib_per_sec;
  const elapsedNs = stats.median_adjusted_elapsed_ns ?? stats.median_elapsed_ns;
  const medianMs = elapsedNs / entry.iterations_per_sample / 1_000_000;
  const adjusted = typeof stats.median_adjusted_mib_per_sec === "number";
  return { tool: entry.tool_version, throughput, medianMs, adjusted };
}
const borrowedParse = scenario("official", tasks[0].group, "ferrocat-borrowed");
const speed = {
  measured: {
    date: reports.official.generated_at.slice(0, 10),
    machine: reports.official.environment.system_label,
    revision: reports.official.environment.git_sha,
    ferrocat: scenario("official", tasks[0].group, "ferrocat").tool.replace("ferrocat@", ""),
    messages: 10_000,
  },
  borrowedParse: {
    mibPerSec: Math.round(borrowedParse.throughput),
    ms: Number(borrowedParse.medianMs.toFixed(1)),
  },
  tasks: tasks.map((task) => {
    const baseline = scenario(task.ferrocat.report, task.group, task.ferrocat.implementation);
    return {
      id: task.id,
      ferrocatMibPerSec: Math.round(baseline.throughput),
      ferrocatMs: Number(baseline.medianMs.toFixed(1)),
      rows: task.comparators.map((comparator) => {
        const own = scenario(comparator.report, task.group, task.ferrocat.implementation);
        const other = scenario(comparator.report, task.group, comparator.implementation);
        return {
          id: comparator.implementation,
          label: comparator.label,
          tool: other.tool,
          adjusted: other.adjusted,
          ratio: Number((own.throughput / other.throughput).toFixed(2)),
        };
      }),
    };
  }),
};

const samples = {
  note: "Generated by docs/scripts/render-home-samples.mjs. Do not edit.",
  ...specimens,
  conformance,
  speed,
};
const formatted = await format("home-samples.json", JSON.stringify(samples));
assert.equal(formatted.errors.length, 0);
if (check) {
  // A Windows checkout may hold the file with CRLF line endings.
  const committed = await readFile(destination, "utf8");
  assert.equal(
    committed.replaceAll("\r\n", "\n"),
    formatted.code,
    "The homepage samples changed. Run pnpm sample:write and commit the result.",
  );
  console.log("The homepage samples match the workspace code and benchmark reports.");
} else {
  await writeFile(destination, formatted.code);
  console.log("Wrote app/data/home-samples.json from the workspace code.");
}
