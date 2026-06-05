# Downloads and Installation

TranslateR is distributed as portable binary archives. There is no installer in
the MVP release flow.

## Choose a Package

Use the package for your operating system:

- Windows: `translater-windows-x86_64.zip`
- Ubuntu: `translater-ubuntu-x86_64.tar.gz`
- Debian: `translater-debian-x86_64.tar.gz`
- macOS Intel: `translater-macos-x86_64.tar.gz`

Each package contains the app binary and project license files.

## Windows

1. Download `translater-windows-x86_64.zip`.
2. Extract the archive to a folder you control.
3. Run `translater.exe`.

If Windows SmartScreen warns about an unsigned portable binary, confirm that the
file came from the expected GitLab or GitHub release page before running it.

## Ubuntu and Debian

1. Download the Ubuntu or Debian `.tar.gz` package.
2. Extract the archive.
3. Run the `translater` binary from the extracted folder.

Some Linux desktops may require standard GTK, XDG portal, or file-dialog support
for native open/save dialogs.

## macOS

1. Download `translater-macos-x86_64.tar.gz`.
2. Extract the archive.
3. Run the `translater` binary.

The macOS package is currently an unsigned, non-notarized portable binary.
Gatekeeper may show:

```text
"translater" not opened. Apple could not verify "translater" is free of malware
that may harm your Mac or compromise your privacy.
```

That warning is expected for downloaded macOS software that is not signed with
an Apple Developer ID and notarized by Apple. A personal CA certificate does not
satisfy Gatekeeper for public macOS downloads.

For a trusted internal copy, approve the app from System Settings after the
first failed open attempt, or remove the quarantine attribute after verifying
the archive came from the expected release:

```sh
xattr -dr com.apple.quarantine translater
./translater
```

Public macOS releases that open without this warning require Apple Developer ID
signing and Apple notarization.

## Package Contents

Release archives should include:

- TranslateR binary.
- `README.md`.
- `CHANGELOG.md`.
- `LICENSE`.
- `NOTICE.md`.
- `LICENSES/`.

See [Release Process](Release-Process) for release automation details.
