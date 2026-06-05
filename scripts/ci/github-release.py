#!/usr/bin/env python3
import json
import mimetypes
import os
import tempfile
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


def request_json(method: str, url: str, token: str, payload: dict | None = None) -> dict:
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
    with urllib.request.urlopen(req) as resp:
        return json.loads(resp.read().decode("utf-8"))


def github_release(repo: str, tag: str, token: str) -> dict:
    api = f"https://api.github.com/repos/{repo}"
    payload = {
        "tag_name": tag,
        "name": f"TranslateR {tag}",
        "body": f"Portable TranslateR release {tag}.",
        "draft": False,
        "prerelease": "-" in tag,
    }
    try:
        return request_json("POST", f"{api}/releases", token, payload)
    except urllib.error.HTTPError as exc:
        if exc.code != 422:
            raise
    release = request_json("GET", f"{api}/releases/tags/{urllib.parse.quote(tag, safe='')}", token)
    return request_json("PATCH", f"{api}/releases/{release['id']}", token, payload)


def delete_existing_asset(release: dict, asset_name: str, token: str) -> None:
    for asset in release.get("assets", []):
        if asset.get("name") == asset_name:
            request_json("DELETE", asset["url"], token)


def download_gitlab_asset(base_url: str, job_token: str, asset_name: str, dest: Path) -> None:
    url = f"{base_url}/{urllib.parse.quote(asset_name)}"
    req = urllib.request.Request(url, headers={"JOB-TOKEN": job_token})
    with urllib.request.urlopen(req) as resp, dest.open("wb") as out:
        out.write(resp.read())


def upload_github_asset(release: dict, asset_path: Path, token: str) -> None:
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
    with urllib.request.urlopen(req) as resp:
        resp.read()


def main() -> None:
    token = require_env("GITHUB_RELEASE_TOKEN")
    repo = os.environ.get("GITHUB_REPOSITORY", "cpjet64/TranslateR")
    tag = require_env("CI_COMMIT_TAG")
    gitlab_base = (
        f"{require_env('CI_API_V4_URL')}/projects/{require_env('CI_PROJECT_ID')}"
        f"/packages/generic/{require_env('PACKAGE_NAME')}/{urllib.parse.quote(tag, safe='')}"
    )
    job_token = require_env("CI_JOB_TOKEN")

    release = github_release(repo, tag, token)
    with tempfile.TemporaryDirectory(prefix="translater-github-release-") as temp_dir:
        temp_path = Path(temp_dir)
        for asset_name, label in ASSETS:
            asset_path = temp_path / asset_name
            print(f"Downloading {label} from GitLab package registry")
            download_gitlab_asset(gitlab_base, job_token, asset_name, asset_path)
            delete_existing_asset(release, asset_name, token)
            print(f"Uploading {label} to GitHub release {tag}")
            upload_github_asset(release, asset_path, token)


if __name__ == "__main__":
    main()
