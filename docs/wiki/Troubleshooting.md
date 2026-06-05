# Troubleshooting

This page covers common TranslateR problems.

## Patch Context Did Not Match

Error:

```text
patch context did not match active PO file
```

Likely causes:

- The wrong base `.po` file is open.
- The `.po` changed after the translator exported the `.tpatch`.
- Another patch already changed the same entry.
- The `.tpatch` was edited outside TranslateR.

Fix:

1. Confirm the active `.po` is the same file version given to the translator.
2. Apply patches in a deliberate order.
3. Ask the translator to export a new `.tpatch` if the base `.po` changed.
4. Manually resolve the translation only when you understand the conflict.

## Text Shows as Squares

TranslateR bundles Noto fallback fonts and also tries platform fonts. If text
still appears as squares:

- Confirm the latest release is installed.
- Confirm the script is covered by the bundled or platform fonts.
- Try the same text in another system application to verify OS font support.
- Report the language code and sample text so a missing font can be added.

## macOS Says Apple Cannot Verify TranslateR

This is macOS Gatekeeper. The current macOS release is an unsigned,
non-notarized portable binary, so Apple cannot verify it automatically.

For a trusted internal copy, approve the app from System Settings after the
first failed open attempt, or remove the quarantine attribute after verifying
the archive came from the expected release:

```sh
xattr -dr com.apple.quarantine translater
./translater
```

Public macOS releases that avoid this warning require Apple Developer ID signing
and Apple notarization. A personal CA certificate does not satisfy Gatekeeper
for public downloads.

## Missing Plural Forms

Plural forms come from `nplurals=N` in the `.po` header. If the header is wrong,
the editor may show too few or too many plural fields. Ask the maintainer to fix
the `.po` header before distributing it.

## Placeholder Warning

If a `c-format` entry warns about placeholders, compare the source and
translation carefully. Keep placeholders such as `%s`, `%d`, and `%1$s` in the
translation.

## Release Download Missing

If a release page is missing an archive:

- Check the GitLab pipeline for the release commit.
- Confirm the packaging job for that operating system passed.
- Confirm the GitLab and GitHub release jobs completed.
- Use the latest successful release if a newer pipeline is still running.
