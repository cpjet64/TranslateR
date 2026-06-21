#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
from zipfile import ZIP_DEFLATED, ZipFile


ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "release-i18n"
SOURCE_CATALOGS = ROOT / "i18n"
DEST = ROOT / "target" / "artifacts" / "translater-i18n.zip"


def verify_release_catalogs() -> None:
    expected = {path.name for path in SOURCE_CATALOGS.glob("*.po")}
    actual = {path.name for path in SOURCE.glob("*.po")}
    missing = sorted(expected - actual)
    if missing:
        raise SystemExit(
            "release-i18n is missing catalog(s): " + ", ".join(missing)
        )


def main() -> None:
    if not SOURCE.is_dir():
        raise SystemExit("release-i18n was not generated")
    verify_release_catalogs()

    DEST.parent.mkdir(parents=True, exist_ok=True)
    if DEST.exists():
        DEST.unlink()

    with ZipFile(DEST, "w", ZIP_DEFLATED) as archive:
        for path in sorted(SOURCE.rglob("*")):
            if path.is_file():
                archive.write(path, Path("i18n") / path.relative_to(SOURCE))

    print(DEST)


if __name__ == "__main__":
    main()
