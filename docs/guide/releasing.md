# Releasing

Gulfstream releases are designed to be reproducible and to fail before publication when source, documentation, formatting, linting, tests, or package metadata are invalid.

## Prerequisites

A release maintainer needs:

- write access to `gulfstream-rs/gulfstream`;
- permission to create GitHub releases;
- owner access to the `gulfstream` package on crates.io;
- a GitHub Actions secret `CARGO_REGISTRY_TOKEN` in the repository or the `crates-io` environment containing a crates.io API token with `publish-update` (and `publish-new` for the first publication) permission.

Because crates.io only attaches tokens to a package after it exists, the first crate version must be published manually from a clean checkout after all release checks pass:

```bash
cargo login
cargo publish --dry-run
cargo publish
```

After the package exists, store that same token in the `CARGO_REGISTRY_TOKEN` secret used by `publish-crates.yml`. The automated workflow posts the package with this token during an eligible release job. Revoking or rotating the token only affects new releases.

## Prepare a release

1. Update the package version in `Cargo.toml`.
2. Move the release notes in `CHANGELOG.md` under the matching version and date.
3. Confirm public API, OpenAPI, configuration, examples, and VitePress documentation agree.
4. Run the complete local gate:

```bash
make verify
cargo package --list
cargo publish --dry-run
```

5. Commit the release changes and merge them into `main`.
6. Tag the exact release commit using the package version:

```bash
git tag v4.0.0 -m "Gulfstream 4.0.0"
git push origin v4.0.0
```

7. Create a GitHub release for the same tag.

The `publish-crates.yml` workflow verifies that the GitHub tag equals `v` plus the package version. Prereleases run every verification step but are not published to crates.io.

## Failure handling

Do not reuse or move a published version tag. Correct the source, increment the version when necessary, and create a new release. crates.io packages are immutable after publication.
