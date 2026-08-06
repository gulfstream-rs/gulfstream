# Contributing

Repository: [github.com/gulfstream-rs/gulfstream](https://github.com/gulfstream-rs/gulfstream)

Read the repository-level [CONTRIBUTING.md](https://github.com/gulfstream-rs/gulfstream/blob/main/CONTRIBUTING.md) before opening a change.

Run the complete gate:

```bash
make verify
```

It covers formatting, all-target checks, Clippy with warnings denied, tests, Rustdoc with warnings denied, release compilation, crates.io dry-run packaging, web-asset validation, and the VitePress production build.

Keep API and configuration changes synchronized across Rust types, validation, the example TOML, OpenAPI, browser runtime configuration, response examples, and documentation. Do not introduce in-memory substitutes for durable state, ignored errors, undocumented operational constants, or independent browser-only behavior.


Release maintainers must follow the [release guide](guide/releasing.md), including the crates.io trusted-publisher setup and immutable-tag policy.
