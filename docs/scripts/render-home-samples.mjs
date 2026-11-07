/* eslint-disable security/detect-non-literal-fs-filename -- Only fixed, repository-owned paths are read or written. */
/**
 * Writes app/data/home-samples.json, everything the homepage shows besides the
 * CLI audit (render-audit-sample.mjs):
 *
 * - the catalog, ICU, review and runtime specimens, printed by docs/samples,
 *   a small program that depends on the published ferrocat crate from
 *   crates.io at the exact version pinned in docs/samples/Cargo.toml;
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

// The specimens: real output of the pinned, published crate.
const manifest = await readFile(new URL("../samples/Cargo.toml", import.meta.url), "utf8");
const pinned = /^ferrocat = "=([^"]+)"$/m.exec(manifest)?.[1];
assert(pinned, "docs/samples/Cargo.toml must pin ferrocat to one exact version.");
const releases = await readJson("../../.release-please-manifest.json");
const release = releases["."];
if (!check && pinned !== release) {
  console.warn(
    `Rendering with ferrocat ${pinned}; the current release is ${release}. Update the pin in docs/samples/Cargo.toml once ${release} is on crates.io.`,
  );
}
const specimens = JSON.parse(
  execFileSync(
    "cargo",
    [
      "run",
      "--locked",
      "--quiet",
      "--manifest-path",
      "docs/samples/Cargo.toml",
      "--target-dir",
      "target/homepage-samples",
    ],
    { cwd: root, encoding: "utf8" },
  ),
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
  official: await readJson("../../benchmark/results/gettext-official-v1-c3caa27.json"),
  workflows: await readJson("../../benchmark/results/gettext-workflows-ecosystem-v1-c3caa27.json"),
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
  const adjusted = typeof stats.median_adjusted_mib_per_sec === "number";
  return { tool: entry.tool_version, throughput, adjusted };
}
const speed = {
  measured: {
    date: reports.official.generated_at.slice(0, 10),
    machine: reports.official.environment.system_label,
    revision: reports.official.environment.git_sha,
    ferrocat: scenario("official", tasks[0].group, "ferrocat").tool.replace("ferrocat@", ""),
    messages: 10_000,
  },
  tasks: tasks.map((task) => ({
    id: task.id,
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
  })),
};

const samples = {
  note: "Generated by docs/scripts/render-home-samples.mjs. Do not edit.",
  renderedWith: { ferrocat: pinned },
  ...specimens,
  conformance,
  speed,
};
const formatted = await format("home-samples.json", JSON.stringify(samples));
assert.equal(formatted.errors.length, 0);
if (check) {
  assert.equal(
    await readFile(destination, "utf8"),
    formatted.code,
    "The homepage samples changed. Run pnpm sample:write and commit the result.",
  );
  console.log(`The homepage samples match ferrocat ${pinned}, the harness and the reports.`);
} else {
  await writeFile(destination, formatted.code);
  console.log(`Wrote app/data/home-samples.json with ferrocat ${pinned}.`);
}
