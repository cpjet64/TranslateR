# Release Process

GitLab is the primary repository. GitHub is public and mirrored from GitLab.

On every push to GitLab `main`, CI:

1. Runs formatting and tests on Ubuntu, Debian, Windows, and macOS.
2. Computes the next patch tag from the latest `vX.Y.Z` tag.
3. Generates release notes from commits since the previous tag.
4. Builds portable packages:
   - `translater-windows-x86_64.zip`
   - `translater-ubuntu-x86_64.tar.gz`
   - `translater-debian-x86_64.tar.gz`
   - `translater-macos-x86_64.tar.gz`
5. Uploads packages to the GitLab Generic Package Registry.
6. Creates or updates the GitLab release.
7. Creates or updates the GitHub release with the same assets.

Release archives contain the binary, README, MIT license, notice file, and
third-party license files. Runtime fallback fonts are embedded in the binary.
