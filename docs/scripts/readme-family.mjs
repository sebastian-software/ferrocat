/* eslint-disable security/detect-non-literal-fs-filename -- Only the four fixed crate READMEs are read or written. */
import { family, FAMILY_SITE } from "ferramenta-family/registry";
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";

const mode = process.argv[2];
assert(["--check", "--write"].includes(mode), "Use --check or --write.");
const current = family.find((tool) => tool.name === "ferrocat");
assert(current, "Ferrocat is missing from the family registry.");
const siblings = family
  .filter((tool) => tool.name !== current.name)
  .map((tool) => `[${tool.name}](${tool.docs ?? tool.repo}) — ${tool.job}`)
  .join(" · ");
const start = "<!-- ferramenta-family:start -->";
const end = "<!-- ferramenta-family:end -->";
const block = `${start}\n**${current.name}** is part of the [Ferramenta](${FAMILY_SITE}) family — A family of Rust tools.\n\nSiblings: ${siblings}\n${end}`;
for (const crate of ["ferrocat", "ferrocat-po", "ferrocat-icu", "ferrocat-cli"]) {
  const file = new URL(`../../crates/${crate}/README.md`, import.meta.url);
  const content = await readFile(file, "utf8");
  const from = content.indexOf(start);
  const endAt = content.indexOf(end, from);
  const to = endAt + end.length;
  assert(
    from !== -1 && endAt !== -1 && to > from && !content.includes(start, from + 1),
    `${crate}: expected one family block`,
  );
  if (mode === "--write") await writeFile(file, content.slice(0, from) + block + content.slice(to));
  else
    assert.equal(
      content.slice(from, to).replaceAll(/\s+/g, ""),
      block.replaceAll(/\s+/g, ""),
      `${crate}: run scripts/check-readme-family.sh --write`,
    );
}
console.log(
  `Four crate README family blocks ${mode === "--write" ? "written" : "verified"} from the installed registry.`,
);
