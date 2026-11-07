import { family } from "ferramenta-family/registry";

import homeSamples from "../data/home-samples.json";

/* Everything the homepage shows comes from generated files: home-samples.json
   (scripts/render-home-samples.mjs) and audit-sample.json
   (scripts/render-audit-sample.mjs). Neither is edited by hand. */
export const samples = homeSamples;
export { default as audit } from "../data/audit-sample.json";

/** The crate version that rendered the specimens. */
export const renderedWith = samples.renderedWith.ferrocat;

const member = family.find((entry) => entry.name === "ferrocat");
if (!member) throw new Error("Ferrocat is missing from the family registry.");
export const ferrocat = member;

const application = family.find((entry) => entry.name === "palamedes");
if (!application?.runsOn?.includes("ferrocat")) {
  throw new Error("The registry no longer says that Palamedes runs on Ferrocat.");
}
export const palamedes = application;

/** A registry note by its title, for the margin. */
export function registryNote(title: string) {
  const note = ferrocat.editorial?.notes.find((entry) => entry.title === title);
  if (!note) throw new Error(`The registry has no Ferrocat note "${title}".`);
  return note;
}

const NUMBERS = ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine"];
/** Small counts in words, as the reading text writes them. */
export const spelled = (count: number) => NUMBERS[count] ?? count.toLocaleString("en-US");

/** The first letter in lower case, to set a registry phrase inside a sentence. */
export const lowerFirst = (text: string) => `${text.charAt(0).toLowerCase()}${text.slice(1)}`;
