import binding from "../packages/ferrocat/index.js";

const requiredExports = [
  "parsePoJson",
  "stringifyPoJson",
  "compileIcuJson",
  "bindingVersion"
];

for (const key of requiredExports) {
  if (!(key in binding)) {
    throw new Error(`Missing expected Ferrocat export: ${key}`);
  }
}

console.log(
  `Loaded Ferrocat native binding with exports: ${requiredExports.join(", ")}`
);
