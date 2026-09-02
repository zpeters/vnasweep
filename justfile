# vnasweep dev tasks. Run `just` to list them.
#
# Prereqs come from flake.nix — direnv (`.envrc` does `use flake`) loads
# `just` and `nodejs`; Rust/Cargo comes from your own rustup.

default:
    @just --list

# Install JS dependencies. Runs `npm ci` only when node_modules is
# missing or older than package-lock.json.
deps:
    @if [ ! -d node_modules ] || [ package-lock.json -nt node_modules ]; then npm ci && touch node_modules; fi

# Run the Tauri dev server — opens the app with hot reload.
tauri-dev: deps
    npm run tauri dev

# Build the Tauri app release bundle.
tauri-build: deps
    npm run tauri build

# Build the release bundle and install it to /Applications (macOS).
install: tauri-build
    rm -rf /Applications/vnasweep.app
    cp -R target/release/bundle/macos/vnasweep.app /Applications/vnasweep.app
    @echo "Installed to /Applications/vnasweep.app"

# Rust: build the backend crate.
build:
    cargo build --manifest-path src-tauri/Cargo.toml

# Rust: run the test suite.
test:
    cargo test --manifest-path src-tauri/Cargo.toml

# Frontend: type-check the .svelte/.ts sources, then Vite build.
web-build: deps
    npm run check
    npm run build

# Frontend unit tests (vitest).
web-test: deps
    npm test

# Format + lint + type-check + test gate.
check: deps
    cargo fmt --manifest-path src-tauri/Cargo.toml --check
    cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
    cargo test --manifest-path src-tauri/Cargo.toml
    npm run check
    npm test
