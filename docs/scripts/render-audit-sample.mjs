/* eslint-disable security/detect-non-literal-fs-filename -- Only fixed, repository-owned example paths are read or written. */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { format } from "oxfmt";

const root = fileURLToPath(new URL("../../", import.meta.url));
// Git may check text fixtures out with CRLF. Keep their browser representation canonical;
// the CLI still reads the original on-disk bytes on each host.
const normalize = (text) => text.replaceAll("\r\n", "\n");
const source = normalize(await readFile(new URL("../examples/source.po", import.meta.url), "utf8"));
const cases = [];
for (const [id, label, expectedExit] of [
  ["complete", "Ready to ship", 0],
  ["missing", "One translation missing", 1],
]) {
  const target = normalize(
    await readFile(new URL(`../examples/${id}.po`, import.meta.url), "utf8"),
  );
  const command = [
    "run",
    "--quiet",
    "--locked",
    "-p",
    "ferrocat-cli",
    "--",
    "audit",
    "--source-locale",
    "en",
    "--source",
    "docs/examples/source.po",
    "--target",
    `de=docs/examples/${id}.po`,
  ];
  const outputs = [];
  for (const args of [command, [...command, "--format", "json"]]) {
    // No shell: the binary and every argument are fixed local fixture inputs.
    const result = spawnSync("cargo", args, { cwd: root, encoding: "utf8" });
    if (result.error) throw result.error;
    assert.equal(result.status, expectedExit, result.stderr);
    outputs.push(result.stdout);
  }
  cases.push({
    id,
    label,
    target,
    report: normalize(outputs[0]).trim(),
    json: JSON.parse(outputs[1]),
    exitCode: expectedExit,
  });
}
const destination = new URL("../app/data/audit-sample.json", import.meta.url);
const formatted = await format("audit-sample.json", JSON.stringify({ source, cases }));
assert.equal(formatted.errors.length, 0);
if (process.argv.includes("--check")) {
  assert.equal(
    normalize(await readFile(destination, "utf8")),
    normalize(formatted.code),
    "Run pnpm sample:write to refresh the native audit examples.",
  );
  console.log(
    "Both homepage examples match Ferrocat's native text and JSON audit output, including exit codes.",
  );
} else {
  await writeFile(destination, normalize(formatted.code));
}
