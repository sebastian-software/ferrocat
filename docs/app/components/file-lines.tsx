import { Fragment } from "ardo/react";

/**
 * A catalog file as written, one numbered line per line of the file. Long
 * lines wrap under a hanging indent, so a wrapped line never reads as a new
 * one. With `tabs`, each tab shows as a marker: FCL separates its fields with
 * them.
 */
export function FileLines({ text, tabs = false }: { text: string; tabs?: boolean }) {
  const lines = text.endsWith("\n") ? text.slice(0, -1).split("\n") : text.split("\n");
  return (
    <ol className="fc-file">
      {lines.map((line, index) => (
        <li key={`${String(index)}:${line}`}>
          <code>
            {tabs
              ? line.split("\t").map((field, position) => (
                  <Fragment key={`${String(position)}:${field}`}>
                    {position > 0 && (
                      <span className="fc-tab" title="Tab">
                        <span className="fc-hidden">(tab)</span>
                      </span>
                    )}
                    {field}
                  </Fragment>
                ))
              : line}
          </code>
        </li>
      ))}
    </ol>
  );
}
