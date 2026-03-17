#!/usr/bin/env python3

import hashlib
import json
import os
import sys
import time

import polib


def canonicalize(value):
    if isinstance(value, list):
        return [canonicalize(item) for item in value]
    if isinstance(value, dict):
        return {key: canonicalize(value[key]) for key in sorted(value)}
    return value


def digest(value):
    rendered = json.dumps(canonicalize(value), separators=(",", ":"), ensure_ascii=False)
    return hashlib.sha256(rendered.encode("utf-8")).hexdigest()


def should_keep_header(key, value):
    return value != "" and key not in {
        "MIME-Version",
        "X-Generator",
        "Content-Type",
        "Content-Transfer-Encoding",
    }


def normalize_po_summary(parsed):
    headers = []
    for key, value in parsed.metadata.items():
        value = str(value)
        if not should_keep_header(str(key), value):
            continue
        headers.append({"key": str(key), "value": value})
    headers.sort(key=lambda entry: (entry["key"], entry["value"]))

    items = []
    for entry in parsed:
        msgstr = []
        if entry.msgstr:
            msgstr = [entry.msgstr]
        elif entry.msgstr_plural:
            for index in sorted(entry.msgstr_plural, key=lambda value: int(value)):
                msgstr.append(str(entry.msgstr_plural[index]))
        items.append(
            {
                "msgctxt": entry.msgctxt or None,
                "msgid": entry.msgid,
                "msgid_plural": entry.msgid_plural or None,
                "msgstr": msgstr,
                "obsolete": bool(entry.obsolete),
            }
        )
    items.sort(
        key=lambda item: (
            item["msgctxt"] is not None,
            item["msgctxt"] or "",
            item["msgid"],
            item["msgid_plural"] is not None,
            item["msgid_plural"] or "",
            item["msgstr"],
            item["obsolete"],
        )
    )

    return {"headers": headers, "items": items}


def success_response(request, **extra):
    return {
        "implementation": request["implementation"],
        "workload": request["workload"],
        "fixture": request["fixture"],
        "success": True,
        "semantic_digest": extra["semantic_digest"],
        "elapsed_ns": extra["elapsed_ns"],
        "bytes_processed": extra["bytes_processed"],
        "items_processed": extra.get("items_processed"),
        "messages_processed": extra.get("messages_processed"),
        "tool_version": extra["tool_version"],
        "po_summary": extra.get("po_summary"),
        "icu_summary": extra.get("icu_summary"),
        "po_output_path": extra.get("po_output_path"),
    }


def run_polib(request):
    with open(request["po_input_path"], "r", encoding="utf-8") as handle:
        content = handle.read()
    tool_version = f"polib@{getattr(polib, '__version__', 'unknown')}"

    if request["operation"] == "parse":
        summary = None
        started = time.perf_counter_ns()
        for _ in range(request["iterations"]):
            parsed = polib.pofile(content)
            summary = normalize_po_summary(parsed)
        elapsed = time.perf_counter_ns() - started
        return success_response(
            request,
            semantic_digest=digest(summary),
            elapsed_ns=elapsed,
            bytes_processed=len(content.encode("utf-8")) * request["iterations"],
            items_processed=len(summary["items"]) * request["iterations"],
            tool_version=tool_version,
            po_summary=summary if request["capture_artifacts"] else None,
        )

    parsed = polib.pofile(content)
    rendered = ""
    started = time.perf_counter_ns()
    for _ in range(request["iterations"]):
        rendered = str(parsed)
    elapsed = time.perf_counter_ns() - started
    reparsed = polib.pofile(rendered)
    summary = normalize_po_summary(reparsed)
    if request["capture_artifacts"] and request.get("po_output_path"):
        with open(request["po_output_path"], "w", encoding="utf-8") as handle:
            handle.write(rendered)
    return success_response(
        request,
        semantic_digest=digest(summary),
        elapsed_ns=elapsed,
        bytes_processed=len(rendered.encode("utf-8")) * request["iterations"],
        items_processed=len(summary["items"]) * request["iterations"],
        tool_version=tool_version,
        po_output_path=request.get("po_output_path") if request["capture_artifacts"] else None,
    )


def main():
    if "--check" in sys.argv:
        print(f"polib@{getattr(polib, '__version__', 'unknown')}", end="")
        return

    request = json.load(sys.stdin)
    if request["implementation"] != "polib":
        raise RuntimeError(f"unsupported python benchmark implementation: {request['implementation']}")
    result = run_polib(request)
    json.dump(result, sys.stdout, ensure_ascii=False)


if __name__ == "__main__":
    main()
