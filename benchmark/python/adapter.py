#!/usr/bin/env python3

import hashlib
import json
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
        if entry.msgid_plural:
            for index in sorted(entry.msgstr_plural, key=lambda value: int(value)):
                msgstr.append(str(entry.msgstr_plural[index]))
        else:
            msgstr = [str(entry.msgstr)]
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


def item_key(entry):
    return f"{entry.msgctxt or ''}\u0004{entry.msgid}\u0004{entry.msgid_plural or ''}"


def merge_polib_catalog(existing, template):
    existing_active = {}
    for entry in existing:
        if entry.obsolete or not entry.msgid:
            continue
        existing_active[item_key(entry)] = entry

    merged = polib.POFile()
    merged.metadata = dict(existing.metadata)
    template_keys = set()

    for template_entry in template:
        if not template_entry.msgid:
            continue
        key = item_key(template_entry)
        template_keys.add(key)
        existing_entry = existing_active.get(key)

        next_entry = polib.POEntry(
            msgid=template_entry.msgid,
            msgctxt=template_entry.msgctxt,
            msgid_plural=template_entry.msgid_plural,
            msgstr="",
            occurrences=list(template_entry.occurrences),
            comment=template_entry.comment,
            tcomment=template_entry.tcomment,
            flags=list(template_entry.flags),
        )

        if template_entry.msgid_plural:
            next_entry.msgstr_plural = {}
            template_plural = dict(template_entry.msgstr_plural)
            existing_plural = (
                dict(existing_entry.msgstr_plural) if existing_entry is not None else {}
            )
            plural_keys = sorted(
                {int(key) for key in template_plural.keys()}
                | {int(key) for key in existing_plural.keys()}
            )
            if not plural_keys:
                plural_keys = [0]
            for plural_key in plural_keys:
                next_entry.msgstr_plural[plural_key] = str(
                    existing_plural.get(plural_key, template_plural.get(plural_key, ""))
                )
            next_entry.msgstr = ""
        else:
            next_entry.msgstr = existing_entry.msgstr if existing_entry is not None else ""

        merged.append(next_entry)

    for entry in existing:
        if entry.obsolete or not entry.msgid:
            continue
        if item_key(entry) in template_keys:
            continue
        obsolete_entry = polib.POEntry(
            msgid=entry.msgid,
            msgctxt=entry.msgctxt,
            msgid_plural=entry.msgid_plural,
            msgstr=entry.msgstr,
            occurrences=list(entry.occurrences),
            comment=entry.comment,
            tcomment=entry.tcomment,
            flags=list(entry.flags),
            obsolete=True,
        )
        if entry.msgid_plural:
            obsolete_entry.msgstr_plural = dict(entry.msgstr_plural)
        merged.append(obsolete_entry)

    return merged


def run_polib(request):
    tool_version = f"polib@{getattr(polib, '__version__', 'unknown')}"

    if request["operation"] == "parse":
        with open(request["po_input_path"], "r", encoding="utf-8") as handle:
            content = handle.read()
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

    if request["operation"] in {"merge", "update-catalog"}:
        with open(request["existing_po_path"], "r", encoding="utf-8") as handle:
            existing_content = handle.read()
        with open(request["pot_path"], "r", encoding="utf-8") as handle:
            template_content = handle.read()
        rendered = ""
        summary = None
        started = time.perf_counter_ns()
        for _ in range(request["iterations"]):
            merged = merge_polib_catalog(
                polib.pofile(existing_content),
                polib.pofile(template_content),
            )
            rendered = str(merged)
            summary = normalize_po_summary(polib.pofile(rendered))
        elapsed = time.perf_counter_ns() - started
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

    with open(request["po_input_path"], "r", encoding="utf-8") as handle:
        content = handle.read()
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
