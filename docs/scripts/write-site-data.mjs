/* eslint-disable security/detect-non-literal-fs-filename -- Inputs are repository-owned routes and the release manifest. */
import { mkdir, readdir, readFile, writeFile } from "node:fs/promises";
import { format } from "oxfmt";

const routes = new URL("../app/routes/", import.meta.url);
const output = new URL("../app/data/", import.meta.url);
await mkdir(output, { recursive: true });
async function write(name, data) {
  const result = await format(name, JSON.stringify(data));
  if (result.errors.length) throw new Error(`Cannot format ${name}`);
  await writeFile(new URL(name, output), result.code);
}
const manifest = JSON.parse(
  await readFile(new URL("../../.release-please-manifest.json", import.meta.url), "utf8"),
);
const cargo = await readFile(new URL("../../Cargo.toml", import.meta.url), "utf8");
const declared = /^rust-version = "([^"]+)"$/m.exec(cargo)?.[1];
if (!declared) throw new Error("Cargo.toml must declare the minimum supported Rust version.");
const rustVersion = declared.split(".").slice(0, 2).join(".");
await write("site.json", { version: manifest["."], rustVersion });

const sections = [
  "guide",
  "reference",
  "quality",
  "performance",
  "operations",
  "architecture",
  "notes",
  "archive",
];
const routeFiles = await readdir(routes, { recursive: true });
const files = routeFiles.map((file) => file.replaceAll("\\", "/"));
const navigation = [];
for (const id of sections) {
  const pages = [];
  for (const file of files.filter((name) => name.startsWith(`${id}/`) && name.endsWith(".mdx"))) {
    const content = await readFile(new URL(file, routes), "utf8");
    const title =
      /^title: (.+)$/m
        .exec(content)?.[1]
        ?.replaceAll("`", "")
        .replaceAll(/^["']|["']$/g, "") ?? file;
    const order = Number(/^order: (.+)$/m.exec(content)?.[1] ?? 1000);
    const to = `/${file.replace(/(?:\/index)?\.mdx$/, "")}`;
    pages.push({ title, to, order });
  }
  pages.sort((a, b) => a.order - b.order || a.to.localeCompare(b.to, "en"));
  navigation.push({
    id,
    label: id[0].toUpperCase() + id.slice(1),
    to: `/${id}`,
    pages: pages.map(({ title, to }) => ({ title, to })),
  });
}
await write("navigation.json", navigation);
