import { useState } from "ardo/react";
import { FerraBars, FerraSelect } from "ferramenta-family/ferra";

// Rounded figures remain guarded by benchmark/published-numbers.json.
// Plain merge and full catalog update are distinct tasks, not one ranking.
const comparisons = [
  {
    id: "parse",
    label: "Parse PO",
    rows: [
      { name: "Ferrocat · borrowed Rust parser", rate: 725, self: true },
      { name: "pofile-ts · Node.js", rate: 149 },
      { name: "gettext/gettext · PHP", rate: 53 },
      { name: "polib · Python", rate: 22 },
    ],
  },
  {
    id: "merge",
    label: "Plain merge",
    rows: [
      { name: "Ferrocat · plain update", rate: 291, self: true },
      { name: "pofile-ts · Node.js", rate: 43 },
      { name: "gettext/gettext · PHP", rate: 16 },
      { name: "polib · Python", rate: 8.1 },
      { name: "GNU msgmerge · adjusted throughput", rate: 4.8 },
    ],
  },
  {
    id: "update",
    label: "Full catalog update",
    rows: [
      { name: "Ferrocat · ICU checks and placeholder tracking", rate: 150, self: true },
      { name: "pofile-ts · Node.js", rate: 42 },
      { name: "Babel · Python Catalog.update", rate: 8.9 },
      { name: "polib · Python", rate: 8.2 },
      { name: "GNU msgmerge · adjusted throughput", rate: 4.8 },
    ],
  },
];
export function BenchmarkResults() {
  const [selected, setSelected] = useState("parse");
  const current = comparisons.find((entry) => entry.id === selected);
  if (!current) throw new Error("The selected example is missing.");
  return (
    <>
      <FerraSelect
        label="Benchmark task"
        value={selected}
        options={comparisons.map(({ id, label }) => ({ value: id, label }))}
        onChange={(event) => {
          setSelected(event.currentTarget.value);
        }}
      />
      <FerraBars
        caption={`${current.label} · median MiB/s · higher is better`}
        items={current.rows.map((row) => ({
          id: row.name,
          label: row.name,
          value: row.rate,
          displayValue: `${row.rate} MiB/s`,
          highlight: "self" in row && row.self,
        }))}
      >
        Historical Ferrocat 2.2.0 results on an Apple M1 Ultra. Different tools may perform
        different validation and preservation work; read the task notes before treating throughput
        as equivalent work.
      </FerraBars>
    </>
  );
}
