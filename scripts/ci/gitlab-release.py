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
    ("translater-i18n.zip", "TranslateR i18n PO files zip"),
    ("SHA256SUMS", "SHA-256 checksum manifest"),
]


def require_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"missing required environment variable: {name}")
    return value


def gitlab_auth_headers() -> list[tuple[str, dict[str, str]]]:
    headers: list[tuple[str, dict[str, str]]] = []
    ci_job_token = os.environ.get("CI_JOB_TOKEN")
    release_token = os.environ.get("GITLAB_RELEASE_TOKEN")
    if ci_job_token:
        headers.append(("CI_JOB_TOKEN", {"JOB-TOKEN": ci_job_token}))
    if release_token:
        headers.append(("GITLAB_RELEASE_TOKEN", {"PRIVATE-TOKEN": release_token}))
    if not headers:
        raise SystemExit("missing required environment variable: CI_JOB_TOKEN")
    return headers


def request_json(method: str, url: str, payload: dict | list[tuple[str, str]] | None = None) -> dict:
    data = None
    if payload is not None:
        data = urllib.parse.urlencode(payload).encode("utf-8")

    auth_headers = gitlab_auth_headers()
    for index, (auth_name, headers) in enumerate(auth_headers):
        request_headers = dict(headers)
        if data is not None:
            request_headers["Content-Type"] = "application/x-www-form-urlencoded"
        req = urllib.request.Request(url, data=data, headers=request_headers, method=method)
        try:
            with urllib.request.urlopen(req) as resp:
                body = resp.read().decode("utf-8")
                return json.loads(body) if body else {}
        except urllib.error.HTTPError as exc:
            body = exc.read().decode("utf-8", errors="replace")
            if exc.code in {401, 403} and index + 1 < len(auth_headers):
                print(f"GitLab API auth with {auth_name} failed {exc.code}; trying next token")
                continue
            print(f"GitLab API error {exc.code}: {body}")
            raise

    raise RuntimeError("GitLab API request failed without an HTTP response")


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

    payload: list[tuple[str, str]] = [
        ("name", f"TranslateR {tag}"),
        ("tag_name", tag),
        ("description", notes),
        ("ref", require_env("CI_COMMIT_SHA")),
    ]
    for label, url in links:
        payload.append(("assets[links][][name]", label))
        payload.append(("assets[links][][url]", url))

    try:
        request_json("POST", f"{api_base}/releases", payload)
        print(f"Created GitLab release {tag}")
    except urllib.error.HTTPError as exc:
        if exc.code != 409:
            raise
        update_payload = [(key, value) for key, value in payload if key not in {"tag_name", "ref"}]
        request_json("PUT", f"{api_base}/releases/{urllib.parse.quote(tag, safe='')}", update_payload)
        print(f"Updated GitLab release {tag}")


if __name__ == "__main__":
    main()
