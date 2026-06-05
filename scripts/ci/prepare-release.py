#!/usr/bin/env python3
import os
import re
import subprocess
from pathlib import Path


SEMVER_TAG = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")


def git(*args: str) -> str:
    return subprocess.check_output(["git", *args], text=True, encoding="utf-8").strip()


def git_lines(*args: str) -> list[str]:
    output = git(*args)
    return [line for line in output.splitlines() if line.strip()]


def semver_key(tag: str) -> tuple[int, int, int]:
    match = SEMVER_TAG.match(tag)
    if not match:
        return (-1, -1, -1)
    return tuple(int(part) for part in match.groups())


def latest_version_tag(exclude: str | None = None) -> str | None:
    tags = [tag for tag in git_lines("tag", "--list", "v*") if SEMVER_TAG.match(tag)]
    if exclude:
        tags = [tag for tag in tags if tag != exclude]
    if not tags:
        return None
    return sorted(tags, key=semver_key)[-1]


def next_patch_tag(previous: str | None) -> str:
    if not previous:
        return "v0.1.0"
    major, minor, patch = semver_key(previous)
    return f"v{major}.{minor}.{patch + 1}"


def commit_subjects(previous: str | None) -> list[str]:
    rev_range = f"{previous}..HEAD" if previous else "HEAD"
    lines = git_lines("log", "--format=%s", rev_range)
    return [line for line in lines if line and not line.startswith("Merge ")]


def write_env(path: Path, values: dict[str, str]) -> None:
    path.write_text("".join(f"{key}={value}\n" for key, value in values.items()), encoding="utf-8")


def main() -> None:
    subprocess.check_call(["git", "fetch", "--tags", "origin"])
    commit_sha = os.environ["CI_COMMIT_SHA"]
    ci_tag = os.environ.get("CI_COMMIT_TAG", "")

    if ci_tag:
        release_tag = ci_tag
        previous_tag = latest_version_tag(exclude=ci_tag)
        release_mode = "tag"
        release_skip = "false"
    else:
        head_tags = [tag for tag in git_lines("tag", "--points-at", "HEAD") if SEMVER_TAG.match(tag)]
        if head_tags:
            release_tag = sorted(head_tags, key=semver_key)[-1]
            previous_tag = latest_version_tag(exclude=release_tag)
            release_mode = "existing"
            release_skip = "true"
        else:
            previous_tag = latest_version_tag()
            release_tag = next_patch_tag(previous_tag)
            release_mode = "auto"
            release_skip = "false"

    subjects = commit_subjects(previous_tag)
    notes = [
        f"# TranslateR {release_tag}",
        "",
        f"Target commit: `{commit_sha}`",
        "",
    ]
    if previous_tag:
        notes.extend([f"Changes since `{previous_tag}`:", ""])
    else:
        notes.extend(["Initial release contents:", ""])
    if subjects:
        notes.extend(f"- {subject}" for subject in subjects)
    else:
        notes.append("- Release package refresh.")
    notes.append("")

    write_env(
        Path("release.env"),
        {
            "RELEASE_TAG": release_tag,
            "RELEASE_PREVIOUS_TAG": previous_tag or "",
            "RELEASE_MODE": release_mode,
            "RELEASE_SKIP": release_skip,
        },
    )
    Path("release-notes.md").write_text("\n".join(notes), encoding="utf-8")
    print(f"Prepared {release_mode} release {release_tag}")


if __name__ == "__main__":
    main()
