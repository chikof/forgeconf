#!/usr/bin/env bash

set -euo pipefail

if [[ $# -ne 1 ]]; then
    echo "Usage: $0 <new-version>" >&2
    echo "Example: $0 0.2.1" >&2
    exit 64
fi

if ! command -v git >/dev/null 2>&1; then
    echo "git is required but not available in PATH" >&2
    exit 127
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required but not available in PATH" >&2
    exit 127
fi

if ! command -v git-cliff >/dev/null 2>&1; then
    echo "git-cliff is required (https://github.com/orhun/git-cliff)" >&2
    exit 127
fi

if ! cargo set-version --version >/dev/null 2>&1; then
    echo "cargo set-version is required; install it via 'cargo install cargo-edit'" >&2
    exit 127
fi

if ! git diff --quiet --cached --ignore-submodules --exit-code || \
   ! git diff --quiet --ignore-submodules --exit-code; then
    echo "Working tree is dirty. Commit or stash changes before releasing." >&2
    exit 1
fi

INPUT_VERSION="$1"
VERSION="${INPUT_VERSION#v}"
TAG="v${VERSION}"

echo "Preparing release ${TAG}..."

cargo set-version --workspace "${VERSION}"

git cliff --config cliff.toml --tag "${TAG}" -o CHANGELOG.md

cargo +nightly fmt --all
cargo test --workspace --all-features

git add CHANGELOG.md Cargo.lock
git diff --name-only HEAD | grep 'Cargo\.toml' | xargs -r git add
git diff --cached --name-only | grep 'Cargo\.toml' | xargs -r git add

git commit -m "chore(release): prepare ${TAG}"

git tag "${TAG}"

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

git push origin "${CURRENT_BRANCH}"
git push origin "${TAG}"

echo "Release ${TAG} has been pushed. GitHub Actions will publish crates and create release notes."
