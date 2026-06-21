#!/usr/bin/env python3
"""Generate TranslateR's own gettext catalogs from Rust i18n calls."""

from __future__ import annotations

import argparse
import ast
import difflib
import re
import shutil
import sys
from dataclasses import dataclass, field
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SRC_DIR = ROOT / "src"
DEFAULT_OUT_DIR = ROOT / "i18n"
GENERATED_OUTPUTS = {"en.po", "translater.pot"}
RELEASE_COPY_SUFFIXES = {".po", ".md"}


@dataclass(order=True)
class Message:
    context: str | None
    msgid: str
    refs: set[str] = field(default_factory=set, compare=False)


def rust_string_at(text: str, start: int) -> tuple[str, int] | None:
    if start >= len(text) or text[start] != '"':
        return None
    i = start + 1
    escaped = False
    while i < len(text):
        ch = text[i]
        if escaped:
            escaped = False
        elif ch == "\\":
            escaped = True
        elif ch == '"':
            literal = text[start : i + 1]
            try:
                return ast.literal_eval(literal), i + 1
            except (SyntaxError, ValueError):
                return None
        i += 1
    return None


def skip_ws(text: str, index: int) -> int:
    while index < len(text) and text[index].isspace():
        index += 1
    return index


def line_number(text: str, index: int) -> int:
    return text.count("\n", 0, index) + 1


def extract_messages() -> list[Message]:
    found: dict[tuple[str | None, str], Message] = {}
    call_re = re.compile(r"\b(tr|tr_format|tr_ctx|tr_ctx_format)!\s*\(")
    fn_re = re.compile(r"\b(tr|tr_format|tr_ctx|tr_ctx_format)\s*\(")

    for path in sorted(SRC_DIR.rglob("*.rs")):
        rel = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8")
        for match in list(fn_re.finditer(text)) + list(call_re.finditer(text)):
            name = match.group(1)
            index = skip_ws(text, match.end())
            context = None
            first = rust_string_at(text, index)
            if first is None:
                continue
            first_value, index = first
            if name in {"tr_ctx", "tr_ctx_format"}:
                index = skip_ws(text, index)
                if index >= len(text) or text[index] != ",":
                    continue
                index = skip_ws(text, index + 1)
                second = rust_string_at(text, index)
                if second is None:
                    continue
                context = first_value
                msgid = second[0]
            else:
                msgid = first_value

            key = (context, msgid)
            entry = found.setdefault(key, Message(context=context, msgid=msgid))
            entry.refs.add(f"{rel}:{line_number(text, match.start())}")

    return sorted(found.values(), key=lambda item: ((item.context or ""), item.msgid))


def po_quote(value: str) -> str:
    escaped = (
        value.replace("\\", "\\\\")
        .replace("\t", "\\t")
        .replace("\r", "\\r")
        .replace("\n", "\\n")
        .replace('"', '\\"')
    )
    return f'"{escaped}"'


def emit_multiline(prefix: str, value: str) -> list[str]:
    if "\n" not in value:
        return [f"{prefix} {po_quote(value)}"]
    lines = [f'{prefix} ""']
    parts = value.splitlines(keepends=True)
    for part in parts:
        lines.append(po_quote(part))
    return lines


def header(project_version: str, language: str | None) -> list[str]:
    lang = language or ""
    return [
        'msgid ""',
        'msgstr ""',
        po_quote(f"Project-Id-Version: TranslateR {project_version}\n"),
        po_quote("Report-Msgid-Bugs-To: https://github.com/cpjet64/TranslateR/issues\n"),
        po_quote("POT-Creation-Date: YEAR-MO-DA HO:MI+ZONE\n"),
        po_quote("PO-Revision-Date: YEAR-MO-DA HO:MI+ZONE\n"),
        po_quote("Last-Translator: TranslateR contributors\n"),
        po_quote("Language-Team: TranslateR contributors\n"),
        po_quote(f"Language: {lang}\n"),
        po_quote("MIME-Version: 1.0\n"),
        po_quote("Content-Type: text/plain; charset=UTF-8\n"),
        po_quote("Content-Transfer-Encoding: 8bit\n"),
    ]


def render_catalog(messages: list[Message], project_version: str, language: str | None) -> str:
    lines: list[str] = []
    lines.extend(header(project_version, language))
    lines.append("")
    for message in messages:
        for ref in sorted(message.refs):
            lines.append(f"#: {ref}")
        if message.context is not None:
            lines.extend(emit_multiline("msgctxt", message.context))
        lines.extend(emit_multiline("msgid", message.msgid))
        msgstr = message.msgid if language == "en" else ""
        lines.extend(emit_multiline("msgstr", msgstr))
        lines.append("")
    return "\n".join(lines)


def diff_text(path: Path, expected: str) -> str:
    current = path.read_text(encoding="utf-8") if path.exists() else ""
    return "".join(
        difflib.unified_diff(
            current.splitlines(keepends=True),
            expected.splitlines(keepends=True),
            fromfile=str(path),
            tofile=f"{path} (generated)",
        )
    )


def write_or_check(path: Path, text: str, check: bool) -> bool:
    if check:
        if path.exists() and path.read_text(encoding="utf-8") == text:
            return True
        sys.stderr.write(diff_text(path, text))
        return False
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)
    return True


def copy_release_catalogs(out_dir: Path, check: bool) -> list[Path]:
    if check:
        return []

    try:
        same_dir = out_dir.resolve() == DEFAULT_OUT_DIR.resolve()
    except FileNotFoundError:
        same_dir = out_dir.absolute() == DEFAULT_OUT_DIR.absolute()
    if same_dir:
        return []

    copied: list[Path] = []
    if not DEFAULT_OUT_DIR.is_dir():
        return copied

    out_dir.mkdir(parents=True, exist_ok=True)
    for path in sorted(DEFAULT_OUT_DIR.iterdir()):
        if not path.is_file():
            continue
        if path.name in GENERATED_OUTPUTS:
            continue
        if path.suffix.lower() not in RELEASE_COPY_SUFFIXES:
            continue
        destination = out_dir / path.name
        shutil.copyfile(path, destination)
        copied.append(destination)
    return copied


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--release-version", default="0.1.0")
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    messages = extract_messages()
    pot = render_catalog(messages, args.release_version, None)
    en = render_catalog(messages, args.release_version, "en")

    ok = True
    ok &= write_or_check(args.out_dir / "translater.pot", pot, args.check)
    ok &= write_or_check(args.out_dir / "en.po", en, args.check)
    if not ok:
        sys.stderr.write("TranslateR i18n catalog is stale. Run scripts/i18n/generate-translater-po.py.\n")
        return 1

    copied = copy_release_catalogs(args.out_dir, args.check)
    print(f"Generated {len(messages)} TranslateR UI messages in {args.out_dir}")
    if copied:
        print(f"Copied {len(copied)} additional i18n release file(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
