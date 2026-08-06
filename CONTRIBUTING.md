# Contributing to Gulfstream

Thank you for improving Gulfstream. Changes should be focused, documented, tested, and safe to operate.

## Development setup

1. Install the toolchain declared in `rust-toolchain.toml`.
2. Install FFmpeg and FFprobe.
3. Copy `config/gulfstream.example.toml` to `config/gulfstream.toml`.
4. Generate local secrets with `./scripts/generate-secrets.sh`.
5. Run `make verify` before opening a pull request.

Install the repository's pre-commit hook once per clone:

```bash
git config core.hooksPath .githooks
```

The hook runs the fast web and Rust quality gates. `make verify` remains required before submission.

## Required checks

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo publish --dry-run
```

Build the documentation with:

```bash
cd docs
npm ci
npm run docs:build
```

Release maintainers should also follow the [release guide](docs/guide/releasing.md).

## Pull requests

- Explain the user-facing or operational reason for the change.
- Add tests for behavior and database invariants.
- Update OpenAPI, VitePress documentation, configuration examples, and the changelog when their contracts change.
- Do not commit secrets, generated media, local databases, or build artifacts.
- Avoid unrelated formatting or dependency changes.

## Security reports

Do not open public issues for vulnerabilities. Follow `SECURITY.md`.
