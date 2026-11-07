import { useState } from "ardo/react";
import {
  MonoCaption,
  MonoFacing,
  MonoSegmented,
  MonoSheet,
  MonoWide,
} from "ferramenta-family/monografie";

import { FileLines } from "./file-lines";
import { audit } from "./home-data";
import "./catalog-specimens.css";

/**
 * The release audit: a German catalog beside the native CLI's report and exit
 * status, from app/data/audit-sample.json (scripts/render-audit-sample.mjs).
 */
export function AuditSpecimen() {
  const [selected, setSelected] = useState("missing");
  const current = audit.cases.find((entry) => entry.id === selected) ?? audit.cases[0];
  if (!current) throw new Error("The audit sample has no cases.");
  return (
    <MonoWide>
      <div className="fc-toggles">
        <MonoSegmented
          label="Catalog example"
          value={current.id}
          options={audit.cases.map(({ id, label }) => ({ value: id, label }))}
          onChange={setSelected}
        />
      </div>
      <MonoFacing className="fc-facing" aria-live="polite">
        <MonoSheet label="The German catalog" meta={`${current.id}.po`}>
          <FileLines text={current.target} />
        </MonoSheet>
        <MonoSheet label="The command-line result" meta={`exit ${String(current.exitCode)}`}>
          <p className="fc-command">{current.command}</p>
          <ul className="fc-report">
            {current.report.split("\n").map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
          <p className="fc-exit" data-failed={current.exitCode === 0 ? undefined : ""}>
            {current.exitCode === 0
              ? "Exit status 0: the release check passes."
              : `Exit status ${String(current.exitCode)}: the release check fails.`}
          </p>
        </MonoSheet>
      </MonoFacing>
      <MonoCaption>
        Saved runs of the Rust command-line tool against an English source with the same two
        messages.
      </MonoCaption>
      <details className="fc-details">
        <summary>Show the English source and the JSON report</summary>
        <MonoFacing className="fc-facing">
          <MonoSheet label="The English source" meta="source.po">
            <FileLines text={audit.source} />
          </MonoSheet>
          <MonoSheet label="The same result as JSON" meta="--format json">
            <FileLines text={JSON.stringify(current.json, null, 2)} />
          </MonoSheet>
        </MonoFacing>
      </details>
    </MonoWide>
  );
}
