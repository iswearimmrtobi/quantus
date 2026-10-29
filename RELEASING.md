# Releasing

All work happens on `dev`. `master` only receives a merge when there is a new release, and the
release tag goes on that merge commit.

## Steps

1. On `dev`, set the release version (list below) and turn `## [Unreleased]` in `CHANGELOG.md` into
   `## [X.Y.Z] - YYYY-MM-DD`, with a fresh empty `## [Unreleased]` above it. Update the compare links
   at the bottom.
2. Run the *Check* workflow by hand on `dev` with `bundle` on, and look at the packages.
3. Merge `dev` into `master` with a merge commit, no squash: `git switch master && git merge --no-ff dev`.
4. Tag and push: `git tag -a vX.Y.Z -m "Quantus X.Y.Z" && git push origin master vX.Y.Z`. The tag
   starts the *Release build* workflow, which attaches the packages to a draft release.
5. Publish the draft by hand. Pushing to `master` also deploys the docs.
6. Back on `dev`, bump to the next `-dev` version (list below) and merge `master` in, so the tag is
   reachable from `dev`.

## Where the version lives

- `Cargo.toml`, `[workspace.package] version` (both crates inherit it)
- `Cargo.lock` (run `cargo check` after the change)
- `desktop/package.json` and `desktop/package-lock.json`
- `desktop/src-tauri/tauri.conf.json`
- the package file names in `README.md` and `docs/docs/getting-started/install.md`
- the compare and tag links in `CHANGELOG.md`
