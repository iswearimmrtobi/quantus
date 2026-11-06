# Releasing

All work happens on `dev`. `master` only receives a merge when there is a new release, and the
release tag goes on that merge commit.

## Steps

1. On `dev`, set the release version (list below) and turn `## [Unreleased]` in `CHANGELOG.md` into
   `## [X.Y.Z] - YYYY-MM-DD`, with a fresh empty `## [Unreleased]` above it. Update the compare links
   at the bottom. Push `dev`.
2. Open a pull request from `dev` into `master`. The *Check* workflow runs on it with the bundles;
   download the AppImage and the .msi from its run and look at them. Further pushes to `dev`
   run it again.
3. Merge the pull request with **Create a merge commit** (the only merge method the repository
   allows). The *Release* workflow (`release-on-merge.yml`) reads the version from `Cargo.toml`,
   tags the merge commit `vX.Y.Z` and calls *Release build*, which attaches the packages to a
   draft release. Pushing to `master` also deploys the docs.
4. Publish the draft by hand.
5. Back on `dev`, bump to the next `-dev` version (list below) and merge `master` in, so the tag is
   reachable from `dev`.

*Release* only tags a version without `-` whose tag does not exist yet, so a merge without a
version bump releases nothing. Without a pull request the same works by hand:
`git switch master && git merge --no-ff dev && git push origin master`. A `v*` tag pushed by hand
also starts *Release build* on its own.

## Where the version lives

- `Cargo.toml`, `[workspace.package] version` (both crates inherit it)
- `Cargo.lock` (run `cargo check` after the change)
- `desktop/package.json` and `desktop/package-lock.json`
- `desktop/src-tauri/tauri.conf.json`
- the package file names in `README.md` and `docs/docs/getting-started/install.md`
- the compare and tag links in `CHANGELOG.md`
