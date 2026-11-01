import { useState } from "ardo/react";
import { ArdoLink as Link } from "ardo/runtime";
import {
  FerraArticle,
  FerraCode,
  FerraCodePair,
  FerraRenderedCode,
  FerraSelect,
} from "ferramenta-family/ferra";

import sample from "../data/audit-sample.json";

export function AuditExample() {
  const [selected, setSelected] = useState("missing");
  const current = sample.cases.find((entry) => entry.id === selected);
  if (!current) throw new Error("The selected example is missing.");
  return (
    <FerraArticle
      id="audit"
      title="Find the missing words before your users do."
      introduction="Two source messages. One German catalog. A release check that tells you exactly what needs attention."
      meta={
        <Link to="/reference/api-overview#audit-catalogs">Audit checks and report formats</Link>
      }
    >
      <FerraSelect
        label="Catalog example"
        value={selected}
        onChange={(event) => {
          setSelected(event.currentTarget.value);
        }}
        options={sample.cases.map(({ id, label }) => ({ value: id, label }))}
      />
      <FerraCodePair aria-live="polite">
        <FerraCode caption="German catalog" file="de.po">
          {current.target}
        </FerraCode>
        <FerraRenderedCode caption={`Native CLI result · exit ${current.exitCode}`}>
          {current.report.split("\n").map((line) => (
            <p key={line}>{line}</p>
          ))}
        </FerraRenderedCode>
      </FerraCodePair>
      <p>
        These are saved runs of the real Rust CLI. The audit compares the catalog with an English
        source containing the same two keys. JSON diagnostics carry the locale, message identity and
        severity into your CI tooling.
      </p>
      <details>
        <summary>Inspect the source catalog and JSON diagnostic</summary>
        <FerraCodePair>
          <FerraCode caption="English source" file="en.po">
            {sample.source}
          </FerraCode>
          <FerraCode caption="Structured audit output">
            {JSON.stringify(current.json, null, 2)}
          </FerraCode>
        </FerraCodePair>
      </details>
    </FerraArticle>
  );
}
