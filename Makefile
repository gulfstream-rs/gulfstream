.PHONY: fmt check lint test doc build package web docs verify run

fmt:
	cargo fmt --all -- --check

check:
	cargo check --all-targets --all-features

lint:
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test --all-targets --all-features

doc:
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

build:
	cargo build --release

package:
	cargo publish --dry-run

web:
	node scripts/validate-web.mjs

docs:
	cd docs && npm ci --no-audit --no-fund && npm run docs:build

verify: web fmt check lint test doc build package docs

run:
	cargo run --release -- --config "$${GULFSTREAM_CONFIG:-config/gulfstream.toml}"
