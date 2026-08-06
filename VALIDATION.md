# Validation

Gulfstream is configured to enforce the complete release gate with Rust 1.97.1, Node.js 22 or newer, FFmpeg, and FFprobe:

```bash
cargo fmt --all -- --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features
cargo build --release
cargo publish --dry-run
node scripts/validate-web.mjs
cd docs && npm ci --no-audit --no-fund && npm run docs:build
```

The GitHub Actions workflows run those commands before merge, documentation deployment, or crates.io publication.

## Packaging-environment results

The source archive was statically validated with these results:

- all TOML, YAML, JSON, JavaScript, shell, HTML, and CSS inputs parsed or passed their dedicated validators;
- the SQLite migration created all 10 application tables with foreign keys enabled;
- all 97 statically embedded SQL statements prepared successfully against the migrated schema;
- the Axum router and runtime-resolved OpenAPI document matched for all 30 HTTP operations;
- all OpenAPI operation identifiers were present and unique;
- all 56 Rust files passed lexical delimiter and module-path validation;
- all 23 direct runtime dependencies were referenced by source or tests;
- local Markdown links resolved;
- no retired service name, unfinished macro, dead-code allowance, or long-lived crates.io token reference remained;
- a real FFmpeg invocation generated a playable fMP4 HLS playlist, initialization segment, and media segment.

Rust/Cargo, rustfmt, Clippy, and installed VitePress dependencies were not available in the packaging environment. Consequently, the Cargo compilation, formatter, Clippy, Rust test, Rustdoc, release-build, package, and VitePress production-build commands could not be executed locally. They remain mandatory, fail-fast GitHub Actions gates. A valid `Cargo.lock` should be generated and committed by the first successful Cargo run before treating a Git checkout as a reproducible application build.
