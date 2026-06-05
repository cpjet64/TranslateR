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

The MVP package is a raw portable binary archive, not a signed `.app` bundle.
macOS may require an explicit approval the first time the binary is opened.

## Package Contents

Release archives should include:

- TranslateR binary.
- `README.md`.
- `CHANGELOG.md`.
- `LICENSE`.
- `NOTICE.md`.
- `LICENSES/`.

See [Release Process](Release-Process) for release automation details.
