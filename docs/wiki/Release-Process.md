# Release Process

GitLab is the primary repository. GitHub is a public mirror. Normal pushes
validate the code and may mirror `main` to GitHub when mirror credentials are
configured. The user decides when to cut releases.

## CI Matrix

The GitLab pipeline validates TranslateR on the self-hosted runner matrix:

- Windows 11
- Ubuntu 24
- Debian 12
- macOS Sequoia Intel

The pipeline runs formatting and tests before packaging. Release jobs run only
from an explicit release tag after validation jobs pass.

## User-Controlled Release Flow

When the user chooses to cut a release, they create or push an explicit
`vX.Y.Z` tag. The release pipeline:

1. Runs formatting and tests on all configured operating systems.
2. Uses the explicit `vX.Y.Z` tag as the release version.
3. Finds the previous `vX.Y.Z` tag.
4. Generates release notes from commit subjects since the previous tag.
5. Builds portable packages.
6. Signs the Windows `translater.exe` with Authenticode on the protected
   Windows runner.
7. Uploads packages to the GitLab Generic Package Registry.
8. Creates or updates the GitLab release.
9. Creates or updates the matching GitHub release and uploads the same assets.

Protected `v*` tags should be created only when the user chooses to cut a
release. Agents must not create releases, tags, package publishes, or
deployments unless the user explicitly instructs it.

## Release Assets

Release packages are portable archives:

- `translater-windows-x86_64.zip`
- `translater-ubuntu-x86_64.tar.gz`
- `translater-debian-x86_64.tar.gz`
- `translater-macos-x86_64.tar.gz`

The release also publishes a standalone interface catalog bundle so translators
can update the UI catalogs without unpacking a platform archive:

- `translater-i18n.zip`

Each platform archive should include:

- TranslateR binary.
- `README.md`.
- `CHANGELOG.md`.
- `LICENSE`.
- `NOTICE.md`.
- `LICENSES/`.
- `i18n/` interface translation catalogs.

Runtime fallback fonts are embedded into the binary. Font license files are
included in `LICENSES/`.

The Windows archive contains an Authenticode-signed `translater.exe` when built
by the protected GitLab release pipeline. Signing is performed before the binary
is copied into the release archive.

## macOS Signing Status

The macOS archive is currently unsigned and non-notarized. It is suitable for
trusted internal testing, but macOS Gatekeeper can block it after download.

Opening without Gatekeeper warnings requires Apple Developer ID signing and
Apple notarization. A personal CA certificate is not enough for public macOS
downloads.

## Required CI Variables

The GitHub mirror and release flow depends on protected CI variables:

- `GITHUB_MIRROR_URL`: SSH URL of the GitHub repository.
- `GITHUB_MIRROR_SSH_KEY`: private deploy key with write access to GitHub.
- `GITHUB_RELEASE_TOKEN`: GitHub token with release permissions.
- `GITLAB_RELEASE_TOKEN`: optional fallback GitLab token with permission to
  create protected `v*` tags and GitLab releases if the built-in CI job token is
  unavailable.
- `CURTPME_SIGNER_URL`: CurtPME signing service URL.
- `CURTPME_SIGNER_TOKEN`: CurtPME signing service bearer token.

## Wiki Sync

The canonical wiki source lives in `docs/wiki/` in the main repository. Hosted
wiki repositories should be synced from those files:

- GitLab wiki branch: `main`.
- GitHub wiki branch: `master`.

After changing wiki source files, push both hosted wiki repositories so the
GitLab and GitHub wiki pages match.

## Release Verification Checklist

After a release pipeline completes:

- Confirm the GitLab pipeline is green.
- Confirm GitLab and GitHub `main` point at the expected commit.
- Confirm the new tag points at the expected release commit.
- Confirm all four platform archives and `translater-i18n.zip` are present on
  GitLab.
- Confirm all four platform archives and `translater-i18n.zip` are present on
  GitHub.
- Spot-check archive contents when packaging changes.
