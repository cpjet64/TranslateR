#!/usr/bin/env python3
import json
import os
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path


ASSETS = [
    ("translater-windows-x86_64.zip", "Windows x86_64 zip"),
    ("translater-ubuntu-x86_64.tar.gz", "Ubuntu x86_64 tar.gz"),
    ("translater-debian-x86_64.tar.gz", "Debian x86_64 tar.gz"),
    ("translater-macos-x86_64.tar.gz", "macOS x86_64 tar.gz"),
]


def require_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"missing required environment variable: {name}")
    return value


def request_json(method: str, url: str, payload: dict | None = None) -> dict:
    data = None
    headers = {"JOB-TOKEN": require_env("CI_JOB_TOKEN")}
    if payload is not None:
        data = urllib.parse.urlencode(payload, doseq=True).encode("utf-8")
        headers["Content-Type"] = "application/x-www-form-urlencoded"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    with urllib.request.urlopen(req) as resp:
        body = resp.read().decode("utf-8")
        return json.loads(body) if body else {}


def main() -> None:
    if os.environ.get("RELEASE_SKIP") == "true":
        print(f"Skipping GitLab release because {require_env('RELEASE_TAG')} already points at HEAD")
        return
    tag = require_env("RELEASE_TAG")
    api_base = f"{require_env('CI_API_V4_URL')}/projects/{require_env('CI_PROJECT_ID')}"
    package_base = f"{api_base}/packages/generic/{require_env('PACKAGE_NAME')}/{urllib.parse.quote(tag, safe='')}"
    notes = Path("release-notes.md").read_text(encoding="utf-8")
    links: list[tuple[str, str]] = [
        (label, f"{package_base}/{asset_name}") for asset_name, label in ASSETS
    ]

    payload: dict[str, str | list[str]] = {
        "name": f"TranslateR {tag}",
        "tag_name": tag,
        "description": notes,
        "ref": require_env("CI_COMMIT_SHA"),
    }
    for idx, (label, url) in enumerate(links):
        payload[f"assets[links][{idx}][name]"] = label
        payload[f"assets[links][{idx}][url]"] = url

    try:
        request_json("POST", f"{api_base}/releases", payload)
        print(f"Created GitLab release {tag}")
    except urllib.error.HTTPError as exc:
        if exc.code != 409:
            raise
        update_payload = {key: value for key, value in payload.items() if key != "tag_name" and key != "ref"}
        request_json("PUT", f"{api_base}/releases/{urllib.parse.quote(tag, safe='')}", update_payload)
        print(f"Updated GitLab release {tag}")


if __name__ == "__main__":
    main()
