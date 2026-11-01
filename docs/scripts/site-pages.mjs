/* eslint-disable security/detect-non-literal-fs-filename -- Route discovery reads only the fixed local app/routes directory. */
import { readdirSync } from "node:fs";

export const expectedPages = [
  "index.html",
  ...readdirSync(new URL("../app/routes/", import.meta.url), { recursive: true })
    .filter((file) => file.endsWith(".mdx"))
    .map((file) => `${file.replace(/(?:\/index)?\.mdx$/, "")}/index.html`),
].sort();
export function pagePath(page) {
  return page === "index.html" ? "/" : `/${page.replace(/index\.html$/, "")}`;
}
