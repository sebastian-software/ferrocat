import { useState } from "ardo/react";
import {
  MonoCaption,
  MonoFacing,
  MonoProofs,
  MonoSegmented,
  MonoSheet,
  MonoWide,
} from "ferramenta-family/monografie";

import { FileLines } from "./file-lines";
import { samples, spelled } from "./home-data";
import "./catalog-specimens.css";

/* The files and reports come from the workspace's Ferrocat code through
   app/data/home-samples.json. */

const { update, storage, icu, review, runtime } = samples;

const plural = (count: number, one: string, many: string) =>
  `${spelled(count)} ${count === 1 ? one : many}`;

/** The German catalog before and after the update, switchable. */
function UpdatedCatalog() {
  const [state, setState] = useState<"after" | "before">("after");
  return (
    <MonoSheet
      label={state === "after" ? "The German catalog, updated" : "The German catalog, before"}
      meta="update_catalog()"
    >
      <div className="fc-sheet-toggle">
        <MonoSegmented
          label="Catalog state"
          value={state}
          options={[
            { value: "after", label: "After the update" },
            { value: "before", label: "Last release" },
          ]}
          onChange={(value) => {
            setState(value === "before" ? "before" : "after");
          }}
        />
      </div>
      <div aria-live="polite">
        <FileLines text={state === "after" ? update.after : update.before} />
      </div>
    </MonoSheet>
  );
}

/** The update: what the code says now, beside the German catalog before and after. */
export function UpdateSpecimen() {
  const { stats } = update;
  return (
    <MonoWide>
      <MonoFacing className="fc-facing">
        <MonoSheet label="What the code says now" meta="SourceExtractedMessage">
          <ul className="fc-messages">
            {update.extracted.map((message) => (
              <li key={message.msgid}>
                <code>{message.msgid}</code>
                <small>
                  {message.msgctxt === null ? "" : `context “${message.msgctxt}”, `}
                  from {message.origin}
                </small>
              </li>
            ))}
          </ul>
        </MonoSheet>
        <UpdatedCatalog />
      </MonoFacing>
      <MonoCaption>
        Ferrocat adds {plural(stats.added, "message", "messages")} with an empty translation, keeps{" "}
        {plural(stats.unchanged, "message", "messages")} with
        {stats.unchanged === 1 ? " its" : " their"} translation, and marks{" "}
        {plural(stats.obsoleteMarked, "message", "messages")} the code no longer uses as obsolete
        and records the date.
      </MonoCaption>
    </MonoWide>
  );
}

/** The same finished catalog as PO and as FCL. */
export function StorageSpecimen() {
  const [format, setFormat] = useState<"fcl" | "po">("fcl");
  const fcl = format === "fcl";
  return (
    <MonoWide>
      <div className="fc-toggles">
        <MonoSegmented
          label="Catalog format"
          value={format}
          options={[
            { value: "fcl", label: "FCL for Git" },
            { value: "po", label: "PO for translation tools" },
          ]}
          onChange={(value) => {
            setFormat(value === "po" ? "po" : "fcl");
          }}
        />
      </div>
      <MonoFacing className="fc-single">
        <MonoSheet
          label={`The German catalog, ${String(storage.messages)} messages`}
          meta={fcl ? "de.fcl, convert_catalog()" : "de.po, update_catalog()"}
        >
          <div aria-live="polite">
            <FileLines text={fcl ? storage.fcl : storage.po} tabs={fcl} />
          </div>
        </MonoSheet>
      </MonoFacing>
      <MonoCaption>
        In FCL each message occupies one line. The{" "}
        <span className="fc-tab-sample" aria-hidden="true" /> marks a tab between fields. A context
        field of <code>n</code> means no context, while <code>s</code> prefixes a context string.
        The <code>r=</code> and <code>c=</code> fields give the source and translator note.
      </MonoCaption>
    </MonoWide>
  );
}

/** One source message and three translations, compared structure against structure. */
export function IcuSpecimen() {
  const [selected, setSelected] = useState(icu.cases[0]?.id ?? "");
  const current = icu.cases.find((entry) => entry.id === selected) ?? icu.cases[0];
  if (!current) throw new Error("The ICU sample has no cases.");
  return (
    <MonoWide>
      <div className="fc-toggles">
        <MonoSegmented
          label="German translation"
          value={current.id}
          options={icu.cases.map(({ id, label }) => ({ value: id, label }))}
          onChange={setSelected}
        />
      </div>
      <MonoFacing className="fc-facing">
        <MonoSheet label="The message" meta="parse_icu()">
          <p className="fc-sub">English source</p>
          <p className="fc-message">{icu.source}</p>
          <p className="fc-sub">German translation</p>
          <p className="fc-message" aria-live="polite">
            {current.translation}
          </p>
        </MonoSheet>
        <MonoSheet label="What Ferrocat reports" meta="compare_icu_messages()">
          <div aria-live="polite">
            {current.diagnostics.length === 0 ? (
              <p className="fc-clear">
                No diagnostics. The translation keeps the source’s structure.
              </p>
            ) : (
              <ul className="fc-findings">
                {current.diagnostics.map((diagnostic) => (
                  <li key={`${diagnostic.code}:${diagnostic.name}`}>
                    <span className="fc-severity">{diagnostic.severity}</span>
                    <code>{diagnostic.code}</code>
                    <p>{diagnostic.message}</p>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </MonoSheet>
      </MonoFacing>
      <MonoCaption>Choose a translation above to see what Ferrocat reports.</MonoCaption>
    </MonoWide>
  );
}

/** A machine translation, a reviewer's correction, and the review report. */
export function ReviewSpecimen() {
  const { summary } = review;
  return (
    <MonoWide>
      <MonoFacing className="fc-facing">
        <MonoSheet label="Written by machine translation" meta="de.fcl">
          <FileLines text={review.before} tabs />
        </MonoSheet>
        <MonoSheet label="After a reviewer’s correction" meta="de.fcl">
          <FileLines text={review.after} tabs />
        </MonoSheet>
      </MonoFacing>
      <MonoProofs
        className="fc-proofs"
        items={[
          {
            label: "Translations changed since the last state",
            value: summary.translation_changed,
          },
          {
            label: "Machine translations still as written",
            value: summary.machine_translation_current,
          },
          {
            label: "Machine translations a person has changed",
            value: summary.machine_translation_stale,
          },
        ]}
      />
      <MonoCaption>
        <code>review_catalogs()</code> compares the two states. The fingerprint (<code>lock=</code>)
        still belongs to “{review.machine}”, so Ferrocat reports the new value as a person’s change.
      </MonoCaption>
    </MonoWide>
  );
}

/** The German catalog with one gap, and the runtime artifact compiled from it. */
export function RuntimeSpecimen() {
  const { artifact } = runtime;
  const missing = artifact.missing[0];
  return (
    <MonoWide>
      <MonoFacing className="fc-facing">
        <MonoSheet label="The German catalog" meta="de.po">
          <FileLines text={runtime.german} />
        </MonoSheet>
        <MonoSheet label="What the application loads" meta="compile_catalog_artifact()">
          <FileLines text={JSON.stringify(artifact, null, 2)} />
        </MonoSheet>
      </MonoFacing>
      <MonoCaption>
        Ferrocat compiles the German catalog with English as the fallback.
        {missing !== undefined && (
          <>
            {" "}
            The cart message has no German translation yet, so it falls back to the{" "}
            {missing.resolved_locale === "en" ? "English" : missing.resolved_locale} source and is
            listed under <code>missing</code>.
          </>
        )}
      </MonoCaption>
    </MonoWide>
  );
}
