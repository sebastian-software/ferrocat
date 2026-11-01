import { defineArdoConfig } from "ardo/config";

export default defineArdoConfig({
  title: "Ferrocat",
  description:
    "Rust-native translation catalogs: parse, update, review, audit and compile PO and ICU messages.",
  siteUrl: "https://ferrocat.dev",
  icons: false,
  markdown: { headingAttributes: true },
  linkCheck: { level: "error" },
});
