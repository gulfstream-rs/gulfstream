## Summary

Describe the problem and the chosen solution.

## Validation

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo check --all-targets --all-features`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo test --all-targets --all-features`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
- [ ] Documentation and OpenAPI updated where required
- [ ] No secrets, generated media, or local databases included

## Operational impact

Describe migrations, configuration changes, rollout steps, or state that there are none.
