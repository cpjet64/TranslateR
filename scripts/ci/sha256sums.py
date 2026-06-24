#!/usr/bin/env python3
import hashlib
from pathlib import Path


ARTIFACTS = [
    Path("ci-artifacts/translater-windows-x86_64.zip"),
    Path("target/artifacts/translater-ubuntu-x86_64.tar.gz"),
    Path("target/artifacts/translater-debian-x86_64.tar.gz"),
    Path("target/artifacts/translater-macos-x86_64.tar.gz"),
    Path("target/artifacts/translater-i18n.zip"),
]


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as file:
        for chunk in iter(lambda: file.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    output = Path("target/artifacts/SHA256SUMS")
    output.parent.mkdir(parents=True, exist_ok=True)

    lines = []
    for artifact in ARTIFACTS:
        if not artifact.is_file():
            raise SystemExit(f"missing release artifact: {artifact}")
        lines.append(f"{sha256_file(artifact)}  {artifact.name}\n")

    output.write_text("".join(lines), encoding="utf-8")
    print(f"Wrote {output}")


if __name__ == "__main__":
    main()
