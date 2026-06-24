#!/usr/bin/env python3
import json
import mimetypes
import os
import shutil
import subprocess
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any


ASSETS = [
    ("translater-windows-x86_64.zip", "Windows x86_64 zip"),
    ("translater-ubuntu-x86_64.tar.gz", "Ubuntu x86_64 tar.gz"),
    ("translater-debian-x86_64.tar.gz", "Debian x86_64 tar.gz"),
    ("translater-macos-x86_64.tar.gz", "macOS x86_64 tar.gz"),
    ("translater-i18n.zip", "TranslateR i18n PO files zip"),
    ("SHA256SUMS", "SHA-256 checksum manifest"),
]

RETRY_ATTEMPTS = 4
RETRY_HTTP_CODES = {429, 500, 502, 503, 504}


def require_env(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"missing required environment variable: {name}")
    return value


def retry_delay_seconds(attempt: int) -> int:
    return min(2**attempt, 10)


def retrying(label: str, attempt: int, exc: BaseException) -> None:
    delay = retry_delay_seconds(attempt)
    print(f"{label} failed on attempt {attempt + 1}/{RETRY_ATTEMPTS}: {exc}; retrying in {delay}s")
    time.sleep(delay)


def request_json(method: str, url: str, token: str, payload: dict | None = None) -> Any:
    data = None
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {token}",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    for attempt in range(RETRY_ATTEMPTS):
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                body = resp.read().decode("utf-8")
                return json.loads(body) if body else {}
        except urllib.error.HTTPError as exc:
            body = exc.read().decode("utf-8", errors="replace")
            if exc.code in RETRY_HTTP_CODES and attempt + 1 < RETRY_ATTEMPTS:
                retrying(f"GitHub API {method} {url}", attempt, exc)
                continue
            print(f"GitHub API error {exc.code}: {body}")
            raise
        except (ConnectionError, TimeoutError, urllib.error.URLError) as exc:
            if attempt + 1 < RETRY_ATTEMPTS:
                retrying(f"GitHub API {method} {url}", attempt, exc)
                continue
            raise
    raise RuntimeError(f"GitHub API {method} {url} failed without a response")


def release_payload(tag: str, notes: str, draft: bool) -> dict:
    return {
        "tag_name": tag,
        "target_commitish": require_env("CI_COMMIT_SHA"),
        "name": f"TranslateR {tag}",
        "body": notes,
        "draft": draft,
        "prerelease": "-" in tag,
    }


def github_release(repo: str, tag: str, token: str, notes: str) -> dict:
    api = f"https://api.github.com/repos/{repo}"
    payload = release_payload(tag, notes, draft=True)
    try:
        return request_json("POST", f"{api}/releases", token, payload)
    except urllib.error.HTTPError as exc:
        if exc.code != 422:
            raise
    release = request_json("GET", f"{api}/releases/tags/{urllib.parse.quote(tag, safe='')}", token)
    if not release.get("draft", False):
        raise SystemExit(f"GitHub release {tag} already exists and is not a draft; assets cannot be changed safely")
    return request_json("PATCH", f"{api}/releases/{release['id']}", token, payload)


def publish_github_release(release: dict, tag: str, token: str, notes: str) -> None:
    request_json("PATCH", release["url"], token, release_payload(tag, notes, draft=False))


def github_asset_exists(release: dict, asset_name: str, token: str) -> bool:
    assets = request_json("GET", release["assets_url"], token)
    return any(asset.get("name") == asset_name for asset in assets)


def delete_existing_asset(release: dict, asset_name: str, token: str) -> None:
    for asset in release.get("assets", []):
        if asset.get("name") == asset_name:
            request_json("DELETE", asset["url"], token)


def download_gitlab_asset(base_url: str, job_token: str, asset_name: str, dest: Path) -> None:
    url = f"{base_url}/{urllib.parse.quote(asset_name)}"
    req = urllib.request.Request(url, headers={"JOB-TOKEN": job_token})
    for attempt in range(RETRY_ATTEMPTS):
        try:
            with urllib.request.urlopen(req, timeout=120) as resp, dest.open("wb") as out:
                out.write(resp.read())
            return
        except (ConnectionError, TimeoutError, urllib.error.URLError) as exc:
            if dest.exists():
                dest.unlink()
            if attempt + 1 < RETRY_ATTEMPTS:
                retrying(f"Download {asset_name} from GitLab", attempt, exc)
                continue
            raise
    raise RuntimeError(f"Download {asset_name} from GitLab failed without a response")


def upload_github_asset(release: dict, asset_path: Path, token: str) -> None:
    if shutil.which("curl"):
        upload_github_asset_with_curl(release, asset_path, token)
        return
    upload_github_asset_with_urllib(release, asset_path, token)


def upload_github_asset_with_curl(release: dict, asset_path: Path, token: str) -> None:
    upload_url = release["upload_url"].split("{", 1)[0]
    query = urllib.parse.urlencode({"name": asset_path.name})
    content_type = mimetypes.guess_type(asset_path.name)[0] or "application/octet-stream"
    url = f"{upload_url}?{query}"
    with tempfile.TemporaryDirectory(prefix="translater-curl-upload-") as temp_dir:
        temp_path = Path(temp_dir)
        body_path = temp_path / "response.json"
        config_path = temp_path / "curl.conf"
        config_path.write_text(
            "\n".join(
                [
                    'header = "Accept: application/vnd.github+json"',
                    f'header = "Authorization: Bearer {token}"',
                    f'header = "Content-Type: {content_type}"',
                    'header = "X-GitHub-Api-Version: 2022-11-28"',
                ]
            )
            + "\n",
            encoding="utf-8",
        )
        os.chmod(config_path, 0o600)
        for attempt in range(RETRY_ATTEMPTS):
            if body_path.exists():
                body_path.unlink()
            command = [
                "curl",
                "--fail-with-body",
                "--silent",
                "--show-error",
                "--location",
                "--http1.1",
                "--connect-timeout",
                "30",
                "--max-time",
                "300",
                "--request",
                "POST",
                "--config",
                str(config_path),
                "--data-binary",
                f"@{asset_path}",
                "--output",
                str(body_path),
                "--write-out",
                "%{http_code}",
                url,
            ]
            result = subprocess.run(command, check=False, capture_output=True, text=True)
            http_code = result.stdout.strip()
            body = body_path.read_text(encoding="utf-8", errors="replace") if body_path.exists() else ""
            if result.returncode == 0 and http_code.startswith("2"):
                return
            if http_code == "422" and attempt > 0 and github_asset_exists(release, asset_path.name, token):
                print(f"GitHub reported existing asset {asset_path.name} after retry; treating upload as complete")
                return
            failure = result.stderr.strip() or body.strip() or f"curl exited with {result.returncode}"
            if attempt + 1 < RETRY_ATTEMPTS:
                retrying(f"Upload {asset_path.name} to GitHub", attempt, RuntimeError(f"{failure} (HTTP {http_code})"))
                continue
            print(f"GitHub asset upload error {http_code}: {body}")
            raise RuntimeError(f"Upload {asset_path.name} to GitHub failed: {failure}")


def upload_github_asset_with_urllib(release: dict, asset_path: Path, token: str) -> None:
    upload_url = release["upload_url"].split("{", 1)[0]
    query = urllib.parse.urlencode({"name": asset_path.name})
    content_type = mimetypes.guess_type(asset_path.name)[0] or "application/octet-stream"
    headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {token}",
        "Content-Type": content_type,
        "X-GitHub-Api-Version": "2022-11-28",
    }
    req = urllib.request.Request(
        f"{upload_url}?{query}",
        data=asset_path.read_bytes(),
        headers=headers,
        method="POST",
    )
    for attempt in range(RETRY_ATTEMPTS):
        try:
            with urllib.request.urlopen(req, timeout=180) as resp:
                resp.read()
            return
        except urllib.error.HTTPError as exc:
            body = exc.read().decode("utf-8", errors="replace")
            if exc.code == 422 and attempt > 0 and github_asset_exists(release, asset_path.name, token):
                print(f"GitHub reported existing asset {asset_path.name} after retry; treating upload as complete")
                return
            if exc.code in RETRY_HTTP_CODES and attempt + 1 < RETRY_ATTEMPTS:
                retrying(f"Upload {asset_path.name} to GitHub", attempt, exc)
                continue
            print(f"GitHub asset upload error {exc.code}: {body}")
            raise
        except (ConnectionError, TimeoutError, urllib.error.URLError) as exc:
            if attempt + 1 < RETRY_ATTEMPTS:
                retrying(f"Upload {asset_path.name} to GitHub", attempt, exc)
                continue
            raise
    raise RuntimeError(f"Upload {asset_path.name} to GitHub failed without a response")


def main() -> None:
    if os.environ.get("RELEASE_SKIP") == "true":
        print(f"Skipping GitHub release because {require_env('RELEASE_TAG')} already points at HEAD")
        return
    token = require_env("GITHUB_RELEASE_TOKEN")
    repo = os.environ.get("GITHUB_REPOSITORY", "cpjet64/TranslateR")
    tag = require_env("RELEASE_TAG")
    gitlab_base = (
        f"{require_env('CI_API_V4_URL')}/projects/{require_env('CI_PROJECT_ID')}"
        f"/packages/generic/{require_env('PACKAGE_NAME')}/{urllib.parse.quote(tag, safe='')}"
    )
    job_token = require_env("CI_JOB_TOKEN")
    notes = Path("release-notes.md").read_text(encoding="utf-8")

    release = github_release(repo, tag, token, notes)
    with tempfile.TemporaryDirectory(prefix="translater-github-release-") as temp_dir:
        temp_path = Path(temp_dir)
        for asset_name, label in ASSETS:
            asset_path = temp_path / asset_name
            print(f"Downloading {label} from GitLab package registry")
            download_gitlab_asset(gitlab_base, job_token, asset_name, asset_path)
            delete_existing_asset(release, asset_name, token)
            print(f"Uploading {label} to GitHub release {tag}")
            upload_github_asset(release, asset_path, token)
    publish_github_release(release, tag, token, notes)


if __name__ == "__main__":
    main()
